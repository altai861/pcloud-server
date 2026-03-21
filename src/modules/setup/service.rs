use crate::{error::ApiError, modules::setup::dto::SetupInitializeRequest};
use argon2::{
    Argon2,
    password_hash::{PasswordHasher, SaltString},
};
use rand_core::OsRng;
use sqlx::{PgPool, Postgres, Transaction};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

pub async fn is_initialized(pool: &PgPool) -> Result<bool, ApiError> {
    let initialized = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE((SELECT is_initialized FROM system_settings WHERE id = 1), false)",
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to read system initialization state"))?;

    if initialized {
        return Ok(true);
    }

    let admin_count = sqlx::query_scalar::<_, i64>(
        "
        SELECT COUNT(*)
        FROM users u
        JOIN roles r ON r.id = u.role_id
        WHERE r.name = 'admin'
        ",
    )
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to read admin user state"))?;

    Ok(admin_count > 0)
}

pub async fn initialize(pool: &PgPool, payload: SetupInitializeRequest) -> Result<(), ApiError> {
    validate_setup_payload(&payload)?;

    let storage_root = prepare_storage_root(&payload.system.storage_root_path)?;
    let storage_root_str = storage_root.to_string_lossy().to_string();
    let password_hash = hash_password(&payload.admin.password)?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to start setup transaction"))?;

    if is_initialized_tx(&mut tx).await? {
        return Err(ApiError::Conflict(
            "System setup has already been completed".to_owned(),
        ));
    }

    ensure_default_roles(&mut tx).await?;

    let admin_role_id = get_role_id(&mut tx, "admin").await?;
    let admin_exists = admin_user_exists(&mut tx).await?;

    if admin_exists {
        return Err(ApiError::Conflict(
            "Admin user already exists; setup cannot run again".to_owned(),
        ));
    }

    let admin_quota = payload.system.total_storage_limit_bytes.unwrap_or(0);

    let admin_user_id = sqlx::query_scalar::<_, i64>(
        "
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
        ",
    )
    .bind(admin_role_id)
    .bind(payload.admin.username)
    .bind(payload.admin.email)
    .bind(password_hash)
    .bind(payload.admin.full_name)
    .bind(admin_quota)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_database_setup_error)?;

    sqlx::query(
        "
        INSERT INTO system_settings (id, is_initialized, storage_root_path, total_storage_limit_bytes)
        VALUES (1, true, $1, $2)
        ON CONFLICT (id)
        DO UPDATE SET
            is_initialized = EXCLUDED.is_initialized,
            storage_root_path = EXCLUDED.storage_root_path,
            total_storage_limit_bytes = EXCLUDED.total_storage_limit_bytes,
            updated_at = NOW()
        ",
    )
    .bind(storage_root_str)
    .bind(payload.system.total_storage_limit_bytes)
    .execute(&mut *tx)
    .await
    .map_err(map_database_setup_error)?;

    sqlx::query(
        "
        INSERT INTO audit_logs (user_id, action_type, target_type, target_id, description)
        VALUES ($1, 'setup.initialize', 'system_settings', 1, 'Initial system setup completed')
        ",
    )
    .bind(admin_user_id)
    .execute(&mut *tx)
    .await
    .map_err(map_database_setup_error)?;

    tx.commit()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to commit setup transaction"))?;

    Ok(())
}

fn validate_setup_payload(payload: &SetupInitializeRequest) -> Result<(), ApiError> {
    if payload.admin.password != payload.admin.password_confirmation {
        return Err(ApiError::BadRequest(
            "Password confirmation does not match".to_owned(),
        ));
    }

    validate_username(&payload.admin.username)?;
    validate_email(&payload.admin.email)?;
    validate_full_name(&payload.admin.full_name)?;
    validate_password_strength(&payload.admin.password)?;

    if let Some(total_limit) = payload.system.total_storage_limit_bytes {
        if total_limit <= 0 {
            return Err(ApiError::BadRequest(
                "totalStorageLimitBytes must be a positive integer when provided".to_owned(),
            ));
        }
    }

    Ok(())
}

fn validate_username(value: &str) -> Result<(), ApiError> {
    if value.len() < 3 || value.len() > 32 {
        return Err(ApiError::BadRequest(
            "Username length must be between 3 and 32 characters".to_owned(),
        ));
    }

    let valid = value
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
    if value.len() < 12 {
        return Err(ApiError::BadRequest(
            "Password must be at least 12 characters long".to_owned(),
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

fn prepare_storage_root(value: &str) -> Result<PathBuf, ApiError> {
    let path = PathBuf::from(value.trim());

    if !path.is_absolute() {
        return Err(ApiError::BadRequest(
            "storageRootPath must be an absolute path".to_owned(),
        ));
    }

    if path == Path::new("/") {
        return Err(ApiError::BadRequest(
            "storageRootPath cannot be '/'".to_owned(),
        ));
    }

    fs::create_dir_all(&path)
        .map_err(|_| ApiError::BadRequest("storageRootPath cannot be created".to_owned()))?;

    let probe_path = path.join(".pcloud_write_probe");

    let mut probe_file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&probe_path)
        .map_err(|_| ApiError::BadRequest("storageRootPath is not writable".to_owned()))?;

    probe_file
        .write_all(b"ok")
        .map_err(|_| ApiError::BadRequest("storageRootPath is not writable".to_owned()))?;

    fs::remove_file(&probe_path)
        .map_err(|_| ApiError::BadRequest("storageRootPath cleanup failed".to_owned()))?;

    Ok(path)
}

fn hash_password(password: &str) -> Result<String, ApiError> {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| ApiError::internal_with_context("Failed to hash password"))
}

async fn is_initialized_tx(tx: &mut Transaction<'_, Postgres>) -> Result<bool, ApiError> {
    let initialized = sqlx::query_scalar::<_, bool>(
        "SELECT COALESCE((SELECT is_initialized FROM system_settings WHERE id = 1), false)",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_setup_error)?;

    if initialized {
        return Ok(true);
    }

    Ok(admin_user_exists(tx).await?)
}

async fn ensure_default_roles(tx: &mut Transaction<'_, Postgres>) -> Result<(), ApiError> {
    sqlx::query(
        "
        INSERT INTO roles (name, description)
        VALUES
            ('admin', 'System administrator with full permissions'),
            ('user', 'Regular personal cloud user')
        ON CONFLICT (name) DO NOTHING
        ",
    )
    .execute(&mut **tx)
    .await
    .map_err(map_database_setup_error)?;

    Ok(())
}

async fn get_role_id(tx: &mut Transaction<'_, Postgres>, name: &str) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>("SELECT id FROM roles WHERE name = $1")
        .bind(name)
        .fetch_one(&mut **tx)
        .await
        .map_err(map_database_setup_error)
}

async fn admin_user_exists(tx: &mut Transaction<'_, Postgres>) -> Result<bool, ApiError> {
    let count = sqlx::query_scalar::<_, i64>(
        "
        SELECT COUNT(*)
        FROM users u
        JOIN roles r ON r.id = u.role_id
        WHERE r.name = 'admin'
        ",
    )
    .fetch_one(&mut **tx)
    .await
    .map_err(map_database_setup_error)?;

    Ok(count > 0)
}

fn map_database_setup_error(error: sqlx::Error) -> ApiError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            if let Some(constraint) = db_error.constraint() {
                if constraint == "users_username_key" {
                    return ApiError::Conflict("Username is already in use".to_owned());
                }

                if constraint == "users_email_key" {
                    return ApiError::Conflict("Email is already in use".to_owned());
                }
            }

            ApiError::Conflict("Unique constraint violation during setup".to_owned())
        }
        _ => ApiError::internal_with_context("Database error during setup"),
    }
}
