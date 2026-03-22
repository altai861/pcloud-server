use crate::{
    error::ApiError,
    modules::{admin::dto::AdminUserDto, auth::service::AuthenticatedUser},
};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use rand_core::OsRng;
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct CreateUserInput {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub password: String,
    pub password_confirmation: String,
    pub storage_quota_bytes: i64,
}

#[derive(Debug, Clone)]
pub struct UpdateUserInput {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub storage_quota_bytes: i64,
}

#[derive(Debug, FromRow)]
struct AdminUserRow {
    id: i64,
    username: String,
    email: String,
    full_name: String,
    role: String,
    status: String,
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
    created_at_unix_ms: i64,
}

#[derive(Debug, FromRow)]
struct SystemStorageRootRow {
    storage_root_path: String,
}

#[derive(Debug, FromRow)]
struct DeleteUserRow {
    id: i64,
    role: String,
    profile_image_rel_path: Option<String>,
}

pub async fn list_users(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
) -> Result<Vec<AdminUserDto>, ApiError> {
    ensure_admin(current_user)?;

    let rows = sqlx::query_as::<_, AdminUserRow>(
        r#"
        SELECT
            u.id,
            u.username,
            u.email,
            u.full_name,
            r.name AS role,
            u.status,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            (EXTRACT(EPOCH FROM u.created_at) * 1000)::BIGINT AS created_at_unix_ms
        FROM users u
        INNER JOIN roles r ON r.id = u.role_id
        ORDER BY LOWER(u.username), u.id
        "#,
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load users list"))?;

    Ok(rows.into_iter().map(admin_user_row_to_dto).collect())
}

pub async fn create_user(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: CreateUserInput,
) -> Result<AdminUserDto, ApiError> {
    ensure_admin(current_user)?;
    validate_create_user_input(&input)?;
    let password_hash = hash_password(&input.password)?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start user creation transaction")
    })?;

    let user_role_id = get_user_role_id_tx(&mut tx).await?;
    let storage_root = load_system_storage_root_tx(&mut tx).await?;
    let storage_quota_bytes = input.storage_quota_bytes;

    let user_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO users (
            role_id,
            username,
            email,
            password_hash,
            full_name,
            status,
            storage_quota_bytes,
            storage_used_bytes
        )
        VALUES ($1, $2, $3, $4, $5, 'active', $6, 0)
        RETURNING id
        "#,
    )
    .bind(user_role_id)
    .bind(input.username.trim())
    .bind(input.email.trim())
    .bind(password_hash)
    .bind(input.full_name.trim())
    .bind(storage_quota_bytes)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    prepare_user_storage_root(&storage_root.storage_root_path, user_id)?;

    let root_folder_id = sqlx::query_scalar::<_, i64>(
        r#"
        INSERT INTO folders (owner_user_id, parent_folder_id, name, path, is_deleted)
        VALUES ($1, NULL, '/', '/', false)
        RETURNING id
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    sqlx::query(
        r#"
        UPDATE users
        SET root_folder_id = $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(root_folder_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    let row = sqlx::query_as::<_, AdminUserRow>(
        r#"
        SELECT
            u.id,
            u.username,
            u.email,
            u.full_name,
            r.name AS role,
            u.status,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            (EXTRACT(EPOCH FROM u.created_at) * 1000)::BIGINT AS created_at_unix_ms
        FROM users u
        INNER JOIN roles r ON r.id = u.role_id
        WHERE u.id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit user creation transaction")
    })?;

    Ok(admin_user_row_to_dto(row))
}

pub async fn update_user(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    target_user_id: i64,
    input: UpdateUserInput,
) -> Result<AdminUserDto, ApiError> {
    ensure_admin(current_user)?;

    if target_user_id <= 0 {
        return Err(ApiError::BadRequest("Invalid user id".to_owned()));
    }

    validate_update_user_input(&input)?;

    let existing_usage = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT storage_used_bytes
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(target_user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load user storage usage"))?
    .ok_or_else(|| ApiError::BadRequest("User was not found".to_owned()))?;

    if input.storage_quota_bytes < existing_usage {
        return Err(ApiError::BadRequest(
            "storageQuotaBytes cannot be less than current storage usage".to_owned(),
        ));
    }

    let result = sqlx::query(
        r#"
        UPDATE users
        SET username = $2,
            email = $3,
            full_name = $4,
            storage_quota_bytes = $5,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(target_user_id)
    .bind(input.username.trim())
    .bind(input.email.trim())
    .bind(input.full_name.trim())
    .bind(input.storage_quota_bytes)
    .execute(pool)
    .await
    .map_err(map_admin_write_error)?;

    if result.rows_affected() == 0 {
        return Err(ApiError::BadRequest("User was not found".to_owned()));
    }

    load_admin_user_by_id(pool, target_user_id).await
}

pub async fn delete_user(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    target_user_id: i64,
) -> Result<(), ApiError> {
    ensure_admin(current_user)?;

    if target_user_id <= 0 {
        return Err(ApiError::BadRequest("Invalid user id".to_owned()));
    }

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start user deletion transaction")
    })?;

    let target_user = load_delete_user_tx(&mut tx, target_user_id).await?;
    if target_user.role.eq_ignore_ascii_case("admin") {
        return Err(ApiError::BadRequest(
            "Admin users cannot be deleted".to_owned(),
        ));
    }

    let storage_root = load_system_storage_root_tx(&mut tx).await?;

    sqlx::query(
        r#"
        DELETE FROM folder_permissions
        WHERE user_id = $1
           OR granted_by_user_id = $1
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    sqlx::query(
        r#"
        DELETE FROM file_permissions
        WHERE user_id = $1
           OR granted_by_user_id = $1
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    sqlx::query(
        r#"
        DELETE FROM sessions
        WHERE user_id = $1
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    sqlx::query(
        r#"
        UPDATE audit_logs
        SET user_id = NULL
        WHERE user_id = $1
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM users
        WHERE id = $1
        "#,
    )
    .bind(target_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_admin_write_error)?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::BadRequest("User was not found".to_owned()));
    }

    remove_user_owned_filesystem_entries(
        &storage_root.storage_root_path,
        target_user.id,
        target_user.profile_image_rel_path.as_deref(),
    )?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit user deletion transaction")
    })?;

    Ok(())
}

fn ensure_admin(current_user: &AuthenticatedUser) -> Result<(), ApiError> {
    if current_user.user.role.eq_ignore_ascii_case("admin") {
        return Ok(());
    }

    Err(ApiError::Unauthorized(
        "Admin privileges are required".to_owned(),
    ))
}

async fn get_user_role_id_tx(tx: &mut Transaction<'_, Postgres>) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT id
        FROM roles
        WHERE name = 'user'
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve user role"))?
    .ok_or_else(|| ApiError::internal_with_context("Default 'user' role is not configured"))
}

async fn load_system_storage_root_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<SystemStorageRootRow, ApiError> {
    sqlx::query_as::<_, SystemStorageRootRow>(
        r#"
        SELECT storage_root_path
        FROM system_settings
        WHERE id = 1
          AND is_initialized = true
        LIMIT 1
        "#,
    )
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load system storage root"))?
    .ok_or_else(|| ApiError::BadRequest("System is not initialized".to_owned()))
}

async fn load_admin_user_by_id(pool: &PgPool, user_id: i64) -> Result<AdminUserDto, ApiError> {
    let row = sqlx::query_as::<_, AdminUserRow>(
        r#"
        SELECT
            u.id,
            u.username,
            u.email,
            u.full_name,
            r.name AS role,
            u.status,
            u.storage_quota_bytes,
            u.storage_used_bytes,
            (EXTRACT(EPOCH FROM u.created_at) * 1000)::BIGINT AS created_at_unix_ms
        FROM users u
        INNER JOIN roles r ON r.id = u.role_id
        WHERE u.id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load user after update"))?
    .ok_or_else(|| ApiError::BadRequest("User was not found".to_owned()))?;

    Ok(admin_user_row_to_dto(row))
}

async fn load_delete_user_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
) -> Result<DeleteUserRow, ApiError> {
    sqlx::query_as::<_, DeleteUserRow>(
        r#"
        SELECT
            u.id,
            r.name AS role,
            u.profile_image_rel_path
        FROM users u
        INNER JOIN roles r ON r.id = u.role_id
        WHERE u.id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load target user for deletion"))?
    .ok_or_else(|| ApiError::BadRequest("User was not found".to_owned()))
}

fn remove_user_owned_filesystem_entries(
    storage_root_path: &str,
    user_id: i64,
    profile_image_rel_path: Option<&str>,
) -> Result<(), ApiError> {
    let user_root = PathBuf::from(storage_root_path)
        .join("users")
        .join(user_id.to_string());
    if user_root.exists() {
        fs::remove_dir_all(&user_root)
            .map_err(|_| ApiError::internal_with_context("Failed to delete user storage files"))?;
    }

    if let Some(profile_image_path) = profile_image_rel_path {
        let normalized = normalize_app_relative_path(profile_image_path)?;
        let absolute_path = PathBuf::from(storage_root_path)
            .join("app-data")
            .join(normalized);

        if absolute_path.exists() {
            fs::remove_file(&absolute_path).map_err(|_| {
                ApiError::internal_with_context("Failed to delete user profile image file")
            })?;
        }
    }

    Ok(())
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

fn prepare_user_storage_root(storage_root_path: &str, user_id: i64) -> Result<(), ApiError> {
    let user_root = PathBuf::from(storage_root_path)
        .join("users")
        .join(user_id.to_string());

    fs::create_dir_all(&user_root)
        .map_err(|_| ApiError::internal_with_context("Failed to create user storage root"))?;

    Ok(())
}

fn validate_create_user_input(input: &CreateUserInput) -> Result<(), ApiError> {
    validate_username(&input.username)?;
    validate_email(&input.email)?;
    validate_full_name(&input.full_name)?;
    if input.password != input.password_confirmation {
        return Err(ApiError::BadRequest(
            "Password confirmation does not match".to_owned(),
        ));
    }
    validate_password_strength(&input.password)?;

    if input.storage_quota_bytes < 0 {
        return Err(ApiError::BadRequest(
            "storageQuotaBytes must be zero or a positive integer".to_owned(),
        ));
    }

    Ok(())
}

fn validate_update_user_input(input: &UpdateUserInput) -> Result<(), ApiError> {
    validate_username(&input.username)?;
    validate_email(&input.email)?;
    validate_full_name(&input.full_name)?;

    if input.storage_quota_bytes < 0 {
        return Err(ApiError::BadRequest(
            "storageQuotaBytes must be zero or a positive integer".to_owned(),
        ));
    }

    Ok(())
}

fn validate_username(value: &str) -> Result<(), ApiError> {
    let trimmed = value.trim();
    if trimmed.len() < 3 || trimmed.len() > 32 {
        return Err(ApiError::BadRequest(
            "Username length must be between 3 and 32 characters".to_owned(),
        ));
    }

    let valid = trimmed
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == '.');

    if !valid {
        return Err(ApiError::BadRequest(
            "Username can contain only letters, numbers, '.', '-' and '_'".to_owned(),
        ));
    }

    Ok(())
}

fn validate_email(value: &str) -> Result<(), ApiError> {
    let trimmed = value.trim();

    let at_idx = trimmed
        .find('@')
        .ok_or_else(|| ApiError::BadRequest("Email must include '@'".to_owned()))?;

    if at_idx == 0 || at_idx + 1 >= trimmed.len() {
        return Err(ApiError::BadRequest("Email format is invalid".to_owned()));
    }

    let domain = &trimmed[(at_idx + 1)..];
    if !domain.contains('.') {
        return Err(ApiError::BadRequest("Email format is invalid".to_owned()));
    }

    Ok(())
}

fn validate_full_name(value: &str) -> Result<(), ApiError> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.len() > 120 {
        return Err(ApiError::BadRequest(
            "Full name must be between 1 and 120 characters".to_owned(),
        ));
    }

    Ok(())
}

fn validate_password_strength(value: &str) -> Result<(), ApiError> {
    if value.len() < 8 {
        return Err(ApiError::BadRequest(
            "Password must be at least 8 characters long".to_owned(),
        ));
    }

    let has_upper = value.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = value.chars().any(|c| c.is_ascii_lowercase());
    let has_digit = value.chars().any(|c| c.is_ascii_digit());

    if !has_upper || !has_lower || !has_digit {
        return Err(ApiError::BadRequest(
            "Password must include uppercase, lowercase, and numeric characters".to_owned(),
        ));
    }

    Ok(())
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal_with_context("Failed to hash password"))
}

fn map_admin_write_error(error: sqlx::Error) -> ApiError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            ApiError::Conflict("Username or email already exists".to_owned())
        }
        _ => ApiError::internal_with_context("Database error during admin operation"),
    }
}

fn admin_user_row_to_dto(row: AdminUserRow) -> AdminUserDto {
    AdminUserDto {
        id: row.id,
        username: row.username,
        email: row.email,
        full_name: row.full_name,
        role: row.role,
        status: row.status,
        storage_quota_bytes: row.storage_quota_bytes,
        storage_used_bytes: row.storage_used_bytes,
        created_at_unix_ms: row.created_at_unix_ms,
    }
}
