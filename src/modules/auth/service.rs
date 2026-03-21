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

const SESSION_LIFETIME_HOURS: i32 = 12;

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
            u.storage_used_bytes
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
            u.storage_used_bytes
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
        },
    })
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
