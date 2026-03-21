use crate::{
    error::ApiError,
    modules::{auth::service::AuthenticatedUser, storage::dto::StorageEntryDto},
};
use sqlx::{FromRow, PgPool};
use std::{
    ffi::OsStr,
    fs,
    path::{Component, Path, PathBuf},
    time::UNIX_EPOCH,
};

#[derive(Debug, Clone)]
pub struct StorageListQuery {
    pub path: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, FromRow)]
struct SystemStorageRow {
    storage_root_path: String,
    total_storage_limit_bytes: Option<i64>,
}

#[derive(Debug)]
pub struct StorageListResult {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<StorageEntryDto>,
    pub total_storage_limit_bytes: Option<i64>,
    pub total_storage_used_bytes: i64,
    pub user_storage_quota_bytes: i64,
    pub user_storage_used_bytes: i64,
}

pub async fn list_user_storage(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: StorageListQuery,
) -> Result<StorageListResult, ApiError> {
    let settings = load_system_storage_settings(pool).await?;
    let total_storage_used_bytes = load_total_storage_usage(pool).await?;

    let storage_root = PathBuf::from(settings.storage_root_path);
    let user_root = storage_root
        .join("users")
        .join(current_user.user.id.to_string());

    fs::create_dir_all(&user_root)
        .map_err(|_| ApiError::internal_with_context("Failed to ensure user storage root"))?;

    let relative_path = normalize_relative_path(query.path.as_deref().unwrap_or_default())?;
    let requested_path = user_root.join(&relative_path);

    if !requested_path.exists() {
        return Err(ApiError::BadRequest(
            "Requested storage path does not exist".to_owned(),
        ));
    }

    if !requested_path.is_dir() {
        return Err(ApiError::BadRequest(
            "Requested storage path must be a directory".to_owned(),
        ));
    }

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase);

    let mut entries = Vec::new();

    for item in fs::read_dir(&requested_path)
        .map_err(|_| ApiError::internal_with_context("Failed to read storage directory"))?
    {
        let item =
            item.map_err(|_| ApiError::internal_with_context("Failed to read directory item"))?;
        let item_path = item.path();
        let file_name = item.file_name().to_string_lossy().trim().to_owned();

        if file_name.is_empty() {
            continue;
        }

        if let Some(search_term) = &search {
            if !file_name.to_lowercase().contains(search_term) {
                continue;
            }
        }

        let metadata = item
            .metadata()
            .map_err(|_| ApiError::internal_with_context("Failed to read file metadata"))?;

        let is_dir = metadata.is_dir();
        let relative_entry = to_relative_path(&user_root, &item_path)?;

        let size_bytes = if is_dir {
            None
        } else {
            i64::try_from(metadata.len()).ok()
        };

        let modified_at_unix_ms = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| i64::try_from(duration.as_millis()).ok());

        entries.push(StorageEntryDto {
            name: file_name,
            path: to_api_path(&relative_entry),
            entry_type: if is_dir {
                "folder".to_owned()
            } else {
                "file".to_owned()
            },
            size_bytes,
            modified_at_unix_ms,
        });
    }

    entries.sort_by(|left, right| {
        let type_ord = left.entry_type.cmp(&right.entry_type);

        if type_ord == std::cmp::Ordering::Equal {
            left.name.to_lowercase().cmp(&right.name.to_lowercase())
        } else if left.entry_type == "folder" {
            std::cmp::Ordering::Less
        } else {
            std::cmp::Ordering::Greater
        }
    });

    let parent_path = if relative_path.as_os_str().is_empty() {
        None
    } else {
        Some(
            relative_path
                .parent()
                .map(to_api_path)
                .unwrap_or_else(|| "/".to_owned()),
        )
    };

    Ok(StorageListResult {
        current_path: to_api_path(&relative_path),
        parent_path,
        entries,
        total_storage_limit_bytes: settings.total_storage_limit_bytes,
        total_storage_used_bytes,
        user_storage_quota_bytes: current_user.user.storage_quota_bytes,
        user_storage_used_bytes: current_user.user.storage_used_bytes,
    })
}

async fn load_system_storage_settings(pool: &PgPool) -> Result<SystemStorageRow, ApiError> {
    sqlx::query_as::<_, SystemStorageRow>(
        r#"
        SELECT storage_root_path, total_storage_limit_bytes
        FROM system_settings
        WHERE id = 1
          AND is_initialized = true
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load system storage settings"))?
    .ok_or_else(|| ApiError::BadRequest("System is not initialized".to_owned()))
}

async fn load_total_storage_usage(pool: &PgPool) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(storage_used_bytes), 0)::BIGINT FROM users")
        .fetch_one(pool)
        .await
        .map_err(|e| {
            ApiError::internal_with_context(format!("Failed to load storage usage totals: {e}"))
        })
}

fn normalize_relative_path(raw: &str) -> Result<PathBuf, ApiError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() || trimmed == "/" {
        return Ok(PathBuf::new());
    }

    if Path::new(trimmed).is_absolute() {
        return Err(ApiError::BadRequest(
            "Storage path must be relative to your root".to_owned(),
        ));
    }

    let mut result = PathBuf::new();

    for component in Path::new(trimmed).components() {
        match component {
            Component::Normal(value) => {
                if value == OsStr::new(".") {
                    continue;
                }
                result.push(value);
            }
            Component::CurDir => {}
            _ => {
                return Err(ApiError::BadRequest(
                    "Storage path contains invalid path segments".to_owned(),
                ));
            }
        }
    }

    Ok(result)
}

fn to_relative_path(base_root: &Path, absolute: &Path) -> Result<PathBuf, ApiError> {
    absolute
        .strip_prefix(base_root)
        .map(Path::to_path_buf)
        .map_err(|_| ApiError::internal_with_context("Failed to resolve storage path"))
}

fn to_api_path(relative: &Path) -> String {
    let normalized = relative.to_string_lossy().replace('\\', "/");

    if normalized.trim().is_empty() {
        "/".to_owned()
    } else {
        format!("/{normalized}")
    }
}
