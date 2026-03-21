use crate::{
    error::ApiError,
    modules::auth::dto::{AuthUserDto, LoginRequest, LoginResponse},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordVerifier},
};
use axum::http::{HeaderMap, header};
use rand_core::{OsRng, RngCore};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, PgPool};
use std::{
    fs,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

const SESSION_LIFETIME_HOURS: i32 = 12;
const PROFILE_IMAGE_ENDPOINT: &str = "/api/client/me/profile-image";
const MAX_PROFILE_IMAGE_BYTES: usize = 5 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub session_id: i64,
    pub user: AuthUserDto,
}

#[derive(Debug, FromRow)]
struct LoginUserRow {
    id: i64,
    username: String,
    full_name: String,
    role: String,
    password_hash: String,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    profile_image_rel_path: Option<String>,
}

#[derive(Debug, FromRow)]
struct SessionUserRow {
    session_id: i64,
    user_id: i64,
    username: String,
    full_name: String,
    role: String,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    profile_image_rel_path: Option<String>,
}

#[derive(Debug, FromRow)]
struct UserProfileImageRow {
    profile_image_rel_path: Option<String>,
    profile_image_content_type: Option<String>,
}

#[derive(Debug, FromRow)]
struct SystemStorageRootRow {
    storage_root_path: String,
}

pub async fn login(
    pool: &PgPool,
    payload: LoginRequest,
    headers: &HeaderMap,
) -> Result<LoginResponse, ApiError> {
    validate_login_payload(&payload)?;

    let user = sqlx::query_as::<_, LoginUserRow>(
        r#"
        SELECT
            u.id,
            u.username,
            u.full_name,
            r.name AS role,
            u.password_hash,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            u.profile_image_rel_path
        FROM users u
        JOIN roles r ON r.id = u.role_id
        WHERE u.username = $1
          AND u.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(payload.username.trim())
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to fetch user account"))?
    .ok_or_else(|| ApiError::Unauthorized("Invalid username or password".to_owned()))?;

    verify_password(&payload.password, &user.password_hash)?;

    let token = generate_access_token();
    let token_hash = hash_token(&token);
    let user_agent = extract_user_agent(headers);
    let ip_address = extract_ip(headers);

    let expires_at = sqlx::query_scalar::<_, String>(
        r#"
        INSERT INTO sessions (user_id, refresh_token_hash, ip_address, user_agent, expires_at)
        VALUES ($1, $2, $3, $4, NOW() + make_interval(hours => $5))
        RETURNING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
        "#,
    )
    .bind(user.id)
    .bind(token_hash)
    .bind(ip_address)
    .bind(user_agent)
    .bind(SESSION_LIFETIME_HOURS)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to create session"))?;

    Ok(LoginResponse {
        access_token: token,
        token_type: "Bearer".to_owned(),
        expires_at,
        user: AuthUserDto {
            id: user.id,
            username: user.username,
            full_name: user.full_name,
            role: user.role,
            storage_quota_bytes: user.storage_quota_bytes,
            storage_used_bytes: user.storage_used_bytes,
            profile_image_url: profile_image_url_from_rel_path(
                user.profile_image_rel_path.as_deref(),
            ),
        },
    })
}

pub async fn authenticate_headers(
    pool: &PgPool,
    headers: &HeaderMap,
) -> Result<AuthenticatedUser, ApiError> {
    let token = extract_bearer_token(headers)?;
    let token_hash = hash_token(token);

    let row = sqlx::query_as::<_, SessionUserRow>(
        r#"
        SELECT
            s.id AS session_id,
            u.id AS user_id,
            u.username,
            u.full_name,
            r.name AS role,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            u.profile_image_rel_path
        FROM sessions s
        JOIN users u ON u.id = s.user_id
        JOIN roles r ON r.id = u.role_id
        WHERE s.refresh_token_hash = $1
          AND s.revoked_at IS NULL
          AND s.expires_at > NOW()
          AND u.status = 'active'
        LIMIT 1
        "#,
    )
    .bind(token_hash)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to validate session"))?
    .ok_or_else(|| ApiError::Unauthorized("Invalid or expired session".to_owned()))?;

    Ok(AuthenticatedUser {
        session_id: row.session_id,
        user: AuthUserDto {
            id: row.user_id,
            username: row.username,
            full_name: row.full_name,
            role: row.role,
            storage_quota_bytes: row.storage_quota_bytes,
            storage_used_bytes: row.storage_used_bytes,
            profile_image_url: profile_image_url_from_rel_path(
                row.profile_image_rel_path.as_deref(),
            ),
        },
    })
}

pub async fn update_profile_image(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    bytes: &[u8],
    content_type: &str,
) -> Result<AuthUserDto, ApiError> {
    if bytes.is_empty() {
        return Err(ApiError::BadRequest("Image payload is empty".to_owned()));
    }

    if bytes.len() > MAX_PROFILE_IMAGE_BYTES {
        return Err(ApiError::BadRequest(
            "Image is too large. Maximum size is 5 MB".to_owned(),
        ));
    }

    let (normalized_content_type, extension) = normalize_content_type(content_type)?;
    let app_root = load_app_storage_root(pool).await?;
    let profile_images_root = app_root.join("profile-images");

    fs::create_dir_all(&profile_images_root).map_err(|_| {
        ApiError::internal_with_context("Failed to create app profile-image storage")
    })?;

    let unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::internal_with_context("Failed to generate file timestamp"))?
        .as_millis();

    let file_name = format!("user-{}-{unix_ms}.{extension}", current_user.user.id);
    let relative_path = Path::new("profile-images").join(file_name);
    let absolute_path = app_root.join(&relative_path);

    fs::write(&absolute_path, bytes)
        .map_err(|_| ApiError::internal_with_context("Failed to persist profile image"))?;

    let previous_rel_path = sqlx::query_scalar::<_, Option<String>>(
        r#"
        SELECT profile_image_rel_path
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(current_user.user.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to read previous profile image metadata"))?
    .flatten();

    sqlx::query(
        r#"
        UPDATE users
        SET profile_image_rel_path = $2,
            profile_image_content_type = $3,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(current_user.user.id)
    .bind(relative_path.to_string_lossy().replace('\\', "/"))
    .bind(normalized_content_type)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to update profile image metadata"))?;

    if let Some(previous) = previous_rel_path {
        if let Ok(previous_relative) = normalize_app_relative_path(&previous) {
            let previous_absolute = app_root.join(previous_relative);
            if previous_absolute.exists() {
                let _ = fs::remove_file(previous_absolute);
            }
        }
    }

    load_user_by_id(pool, current_user.user.id).await
}

pub async fn read_profile_image(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
) -> Result<(Vec<u8>, String), ApiError> {
    let metadata = sqlx::query_as::<_, UserProfileImageRow>(
        r#"
        SELECT profile_image_rel_path, profile_image_content_type
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(current_user.user.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load profile image metadata"))?
    .ok_or_else(|| ApiError::Unauthorized("Invalid user session".to_owned()))?;

    let relative_path = metadata
        .profile_image_rel_path
        .ok_or_else(|| ApiError::BadRequest("Profile image is not set".to_owned()))?;

    let normalized_relative = normalize_app_relative_path(&relative_path)?;
    let app_root = load_app_storage_root(pool).await?;
    let absolute_path = app_root.join(normalized_relative);

    if !absolute_path.exists() {
        return Err(ApiError::BadRequest(
            "Profile image file does not exist".to_owned(),
        ));
    }

    let bytes = fs::read(absolute_path)
        .map_err(|_| ApiError::internal_with_context("Failed to read profile image file"))?;

    let content_type = metadata
        .profile_image_content_type
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_owned());

    Ok((bytes, content_type))
}

pub async fn revoke_current_session(pool: &PgPool, session_id: i64) -> Result<(), ApiError> {
    sqlx::query(
        r#"
        UPDATE sessions
        SET revoked_at = NOW(), updated_at = NOW()
        WHERE id = $1
          AND revoked_at IS NULL
        "#,
    )
    .bind(session_id)
    .execute(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to revoke session"))?;

    Ok(())
}

async fn load_user_by_id(pool: &PgPool, user_id: i64) -> Result<AuthUserDto, ApiError> {
    #[derive(Debug, FromRow)]
    struct UserRow {
        id: i64,
        username: String,
        full_name: String,
        role: String,
        storage_quota_bytes: i64,
        storage_used_bytes: i64,
        profile_image_rel_path: Option<String>,
    }

    let row = sqlx::query_as::<_, UserRow>(
        r#"
        SELECT
            u.id,
            u.username,
            u.full_name,
            r.name AS role,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            u.profile_image_rel_path
        FROM users u
        JOIN roles r ON r.id = u.role_id
        WHERE u.id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to fetch user profile"))?
    .ok_or_else(|| ApiError::Unauthorized("Invalid user session".to_owned()))?;

    Ok(AuthUserDto {
        id: row.id,
        username: row.username,
        full_name: row.full_name,
        role: row.role,
        storage_quota_bytes: row.storage_quota_bytes,
        storage_used_bytes: row.storage_used_bytes,
        profile_image_url: profile_image_url_from_rel_path(row.profile_image_rel_path.as_deref()),
    })
}

async fn load_app_storage_root(pool: &PgPool) -> Result<PathBuf, ApiError> {
    let row = sqlx::query_as::<_, SystemStorageRootRow>(
        r#"
        SELECT storage_root_path
        FROM system_settings
        WHERE id = 1
          AND is_initialized = true
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load system settings"))?
    .ok_or_else(|| ApiError::BadRequest("System is not initialized".to_owned()))?;

    Ok(PathBuf::from(row.storage_root_path).join("app-data"))
}

fn normalize_app_relative_path(raw: &str) -> Result<PathBuf, ApiError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(ApiError::BadRequest(
            "Invalid profile image storage path".to_owned(),
        ));
    }

    if Path::new(trimmed).is_absolute() {
        return Err(ApiError::BadRequest(
            "Invalid profile image storage path".to_owned(),
        ));
    }

    let mut result = PathBuf::new();

    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(value) => result.push(value),
            Component::CurDir => {}
            _ => {
                return Err(ApiError::BadRequest(
                    "Invalid profile image storage path".to_owned(),
                ));
            }
        }
    }

    if result.as_os_str().is_empty() {
        return Err(ApiError::BadRequest(
            "Invalid profile image storage path".to_owned(),
        ));
    }

    Ok(result)
}

fn normalize_content_type(content_type: &str) -> Result<(&'static str, &'static str), ApiError> {
    let normalized = content_type
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();

    match normalized.as_str() {
        "image/png" => Ok(("image/png", "png")),
        "image/jpeg" => Ok(("image/jpeg", "jpg")),
        "image/webp" => Ok(("image/webp", "webp")),
        "image/gif" => Ok(("image/gif", "gif")),
        _ => Err(ApiError::BadRequest(
            "Unsupported image type. Use PNG, JPEG, WEBP, or GIF".to_owned(),
        )),
    }
}

fn profile_image_url_from_rel_path(path: Option<&str>) -> Option<String> {
    path.filter(|value| !value.trim().is_empty())
        .map(|_| PROFILE_IMAGE_ENDPOINT.to_owned())
}

fn validate_login_payload(payload: &LoginRequest) -> Result<(), ApiError> {
    if payload.username.trim().is_empty() {
        return Err(ApiError::BadRequest("username is required".to_owned()));
    }

    if payload.password.is_empty() {
        return Err(ApiError::BadRequest("password is required".to_owned()));
    }

    Ok(())
}

fn verify_password(password: &str, stored_hash: &str) -> Result<(), ApiError> {
    let parsed_hash = PasswordHash::new(stored_hash)
        .map_err(|_| ApiError::Unauthorized("Invalid username or password".to_owned()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| ApiError::Unauthorized("Invalid username or password".to_owned()))
}

fn extract_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::USER_AGENT)
        .and_then(|value| value.to_str().ok())
        .map(|value| value.to_owned())
}

fn extract_ip(headers: &HeaderMap) -> Option<String> {
    headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .next()
                .unwrap_or_default()
                .trim()
                .to_owned()
        })
        .filter(|value| !value.is_empty())
}

fn extract_bearer_token(headers: &HeaderMap) -> Result<&str, ApiError> {
    let auth = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_owned()))?;

    let (scheme, token) = auth
        .split_once(' ')
        .ok_or_else(|| ApiError::Unauthorized("Malformed Authorization header".to_owned()))?;

    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return Err(ApiError::Unauthorized(
            "Malformed Authorization header".to_owned(),
        ));
    }

    Ok(token.trim())
}

fn hash_token(raw_token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw_token.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn generate_access_token() -> String {
    let mut raw = [0_u8; 32];
    OsRng.fill_bytes(&mut raw);
    hex::encode(raw)
}
