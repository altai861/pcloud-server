use crate::{
    error::ApiError,
    modules::{auth::service::AuthenticatedUser, storage::dto::StorageEntryDto},
};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
};

const MAX_UPLOAD_SIZE_BYTES: i64 = 5 * 1024 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct StorageListQuery {
    pub path: Option<String>,
    pub search: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateFolderInput {
    pub parent_path: Option<String>,
    pub name: String,
}

#[derive(Debug)]
pub struct UploadFileInput {
    pub folder_path: Option<String>,
    pub file_name: String,
    pub content_type: Option<String>,
    pub temp_file_path: PathBuf,
    pub file_size_bytes: i64,
    pub checksum: String,
}

#[derive(Debug, FromRow)]
struct SystemStorageRow {
    storage_root_path: String,
    total_storage_limit_bytes: Option<i64>,
}

#[derive(Debug, FromRow)]
struct FolderRow {
    id: i64,
    path: String,
}

#[derive(Debug, FromRow)]
struct StorageEntryRow {
    name: String,
    path: String,
    entry_type: String,
    size_bytes: Option<i64>,
    modified_at_unix_ms: i64,
}

#[derive(Debug, FromRow)]
struct FolderMetadataRow {
    name: String,
    path: String,
    created_at_unix_ms: i64,
    modified_at_unix_ms: i64,
    folder_count: i64,
    file_count: i64,
}

#[derive(Debug, FromRow)]
struct DownloadFileRow {
    storage_path: String,
    mime_type: Option<String>,
    original_file_name: Option<String>,
    name: String,
}

#[derive(Debug, FromRow)]
struct UserStorageRow {
    storage_quota_bytes: i64,
    storage_used_bytes: i64,
}

#[derive(Debug, FromRow)]
struct DeleteFileRow {
    id: i64,
    size_bytes: i64,
    storage_path: String,
    logical_path: String,
}

#[derive(Debug, FromRow)]
struct DeleteFolderRow {
    path: String,
}

#[derive(Debug, FromRow)]
struct TrashedFolderRestoreRow {
    path: String,
    parent_is_deleted: Option<bool>,
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

#[derive(Debug)]
pub struct StorageFolderMetadataResult {
    pub name: String,
    pub path: String,
    pub created_at_unix_ms: i64,
    pub modified_at_unix_ms: i64,
    pub folder_count: i64,
    pub file_count: i64,
    pub total_item_count: i64,
}

#[derive(Debug)]
pub struct DownloadFileResult {
    pub absolute_path: PathBuf,
    pub mime_type: Option<String>,
    pub download_name: String,
}

#[derive(Debug)]
pub struct DeletedStorageResult {
    pub deleted_path: String,
    pub entry_type: String,
    pub reclaimed_bytes: i64,
}

#[derive(Debug)]
pub struct RestoredStorageResult {
    pub restored_path: String,
    pub entry_type: String,
}

pub async fn list_user_storage(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: StorageListQuery,
) -> Result<StorageListResult, ApiError> {
    let settings = load_system_storage_settings(pool).await?;
    let total_storage_used_bytes = load_total_storage_usage(pool).await?;

    let requested_path = normalize_api_path(query.path.as_deref().unwrap_or("/"))?;
    let current_folder = load_requested_folder(pool, current_user.user.id, &requested_path).await?;

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let storage_entries = load_storage_entries(
        pool,
        current_user.user.id,
        current_folder.id,
        &current_folder.path,
        search,
    )
    .await?;

    let entries = storage_entries
        .into_iter()
        .map(|entry| StorageEntryDto {
            name: entry.name,
            path: normalize_db_path(&entry.path),
            entry_type: entry.entry_type,
            size_bytes: entry.size_bytes,
            modified_at_unix_ms: Some(entry.modified_at_unix_ms),
        })
        .collect();

    let current_path = normalize_db_path(&current_folder.path);

    Ok(StorageListResult {
        current_path: current_path.clone(),
        parent_path: parent_api_path(&current_path),
        entries,
        total_storage_limit_bytes: settings.total_storage_limit_bytes,
        total_storage_used_bytes,
        user_storage_quota_bytes: current_user.user.storage_quota_bytes,
        user_storage_used_bytes: current_user.user.storage_used_bytes,
    })
}

pub async fn list_user_trash(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: StorageListQuery,
) -> Result<StorageListResult, ApiError> {
    let settings = load_system_storage_settings(pool).await?;
    let total_storage_used_bytes = load_total_storage_usage(pool).await?;

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let entries = load_trashed_storage_entries(pool, current_user.user.id, search)
        .await?
        .into_iter()
        .map(storage_entry_row_to_dto)
        .collect();

    Ok(StorageListResult {
        current_path: "/trash".to_owned(),
        parent_path: None,
        entries,
        total_storage_limit_bytes: settings.total_storage_limit_bytes,
        total_storage_used_bytes,
        user_storage_quota_bytes: current_user.user.storage_quota_bytes,
        user_storage_used_bytes: current_user.user.storage_used_bytes,
    })
}

pub async fn get_folder_metadata(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<StorageFolderMetadataResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    let current_folder = load_requested_folder(pool, current_user.user.id, &requested_path).await?;

    let metadata = sqlx::query_as::<_, FolderMetadataRow>(
        r#"
        SELECT
            folder.name,
            folder.path,
            (EXTRACT(EPOCH FROM folder.created_at) * 1000)::BIGINT AS created_at_unix_ms,
            (EXTRACT(EPOCH FROM folder.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
            (
                SELECT COUNT(*)::BIGINT
                FROM folders child
                WHERE child.owner_user_id = $1
                  AND child.parent_folder_id = folder.id
                  AND child.is_deleted = false
            ) AS folder_count,
            (
                SELECT COUNT(*)::BIGINT
                FROM files file_row
                WHERE file_row.owner_user_id = $1
                  AND file_row.folder_id = folder.id
                  AND file_row.is_deleted = false
            ) AS file_count
        FROM folders folder
        WHERE folder.owner_user_id = $1
          AND folder.id = $2
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(current_user.user.id)
    .bind(current_folder.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load folder metadata"))?
    .ok_or_else(|| ApiError::BadRequest("Requested storage path does not exist".to_owned()))?;

    let folder_name = if metadata.path == "/" {
        "/".to_owned()
    } else {
        metadata.name
    };

    Ok(StorageFolderMetadataResult {
        name: folder_name,
        path: normalize_db_path(&metadata.path),
        created_at_unix_ms: metadata.created_at_unix_ms,
        modified_at_unix_ms: metadata.modified_at_unix_ms,
        folder_count: metadata.folder_count,
        file_count: metadata.file_count,
        total_item_count: metadata.folder_count + metadata.file_count,
    })
}

pub async fn resolve_file_download(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<DownloadFileResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Requested path must point to a file".to_owned(),
        ));
    }

    let settings = load_system_storage_settings(pool).await?;

    let file_row = sqlx::query_as::<_, DownloadFileRow>(
        r#"
        SELECT
            file_row.storage_path,
            file_row.mime_type,
            file_row.original_file_name,
            file_row.name
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.owner_user_id = $1
          AND folder.owner_user_id = $1
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
          AND (
              CASE
                  WHEN folder.path = '/' THEN '/' || file_row.name
                  ELSE folder.path || '/' || file_row.name
              END
          ) = $2
        LIMIT 1
        "#,
    )
    .bind(current_user.user.id)
    .bind(&requested_path)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve file download path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?;

    let absolute_path = PathBuf::from(&settings.storage_root_path)
        .join(file_row.storage_path.trim_start_matches('/'));

    if !absolute_path.is_file() {
        return Err(ApiError::BadRequest(
            "Requested file is not available on disk".to_owned(),
        ));
    }

    let download_name = file_row
        .original_file_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .unwrap_or(file_row.name);

    Ok(DownloadFileResult {
        absolute_path,
        mime_type: file_row.mime_type,
        download_name,
    })
}

pub async fn create_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: CreateFolderInput,
) -> Result<StorageEntryDto, ApiError> {
    let folder_name = normalize_item_name(&input.name, "Folder name")?;
    let parent_path = normalize_api_path(input.parent_path.as_deref().unwrap_or("/"))?;
    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start folder creation transaction")
    })?;

    let parent_folder =
        load_requested_folder_tx(&mut tx, current_user.user.id, &parent_path).await?;

    ensure_name_not_taken_tx(
        &mut tx,
        current_user.user.id,
        parent_folder.id,
        &folder_name,
    )
    .await?;

    let child_path = join_child_path(&normalize_db_path(&parent_folder.path), &folder_name);

    let inserted = sqlx::query_as::<_, StorageEntryRow>(
        r#"
        INSERT INTO folders (owner_user_id, parent_folder_id, name, path, is_deleted)
        VALUES ($1, $2, $3, $4, false)
        RETURNING
            name,
            path,
            'folder'::TEXT AS entry_type,
            NULL::BIGINT AS size_bytes,
            (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        "#,
    )
    .bind(current_user.user.id)
    .bind(parent_folder.id)
    .bind(&folder_name)
    .bind(&child_path)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    let user_root = resolve_user_storage_root(&settings.storage_root_path, current_user.user.id);
    let folder_fs_path = user_root.join(logical_path_to_relative_path(&child_path));

    fs::create_dir_all(&folder_fs_path).map_err(|_| {
        ApiError::internal_with_context("Failed to create directory on the storage filesystem")
    })?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit folder creation transaction")
    })?;

    Ok(storage_entry_row_to_dto(inserted))
}

pub async fn upload_file(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: UploadFileInput,
) -> Result<StorageEntryDto, ApiError> {
    let temp_file_path = input.temp_file_path.clone();

    if input.file_size_bytes <= 0 {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::BadRequest("Uploaded file payload is empty".to_owned()),
        ));
    }

    if input.file_size_bytes > MAX_UPLOAD_SIZE_BYTES {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::BadRequest("Uploaded file exceeds the 5 GB limit".to_owned()),
        ));
    }

    let file_name = match normalize_file_name(&input.file_name) {
        Ok(value) => value,
        Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
    };

    let extension = extract_extension(&file_name);
    let folder_path = match normalize_api_path(input.folder_path.as_deref().unwrap_or("/")) {
        Ok(value) => value,
        Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
    };

    let settings = match load_system_storage_settings(pool).await {
        Ok(value) => value,
        Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
    };

    let mut tx = match pool.begin().await {
        Ok(value) => value,
        Err(_) => {
            return Err(cleanup_temp_file_and_return(
                &temp_file_path,
                ApiError::internal_with_context("Failed to start file upload transaction"),
            ));
        }
    };

    let target_folder =
        match load_requested_folder_tx(&mut tx, current_user.user.id, &folder_path).await {
            Ok(value) => value,
            Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
        };

    if let Err(error) =
        ensure_name_not_taken_tx(&mut tx, current_user.user.id, target_folder.id, &file_name).await
    {
        return Err(cleanup_temp_file_and_return(&temp_file_path, error));
    }

    let user_storage = match load_user_storage_tx(&mut tx, current_user.user.id).await {
        Ok(value) => value,
        Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
    };

    if user_storage.storage_quota_bytes > 0
        && user_storage.storage_used_bytes + input.file_size_bytes
            > user_storage.storage_quota_bytes
    {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::Conflict("User storage quota would be exceeded by this upload".to_owned()),
        ));
    }

    if let Some(total_limit) = settings.total_storage_limit_bytes {
        let total_storage_used = match load_total_storage_usage_tx(&mut tx).await {
            Ok(value) => value,
            Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
        };

        if total_storage_used + input.file_size_bytes > total_limit {
            return Err(cleanup_temp_file_and_return(
                &temp_file_path,
                ApiError::Conflict(
                    "System storage limit would be exceeded by this upload".to_owned(),
                ),
            ));
        }
    }

    let user_root = resolve_user_storage_root(&settings.storage_root_path, current_user.user.id);
    let logical_folder_path = normalize_db_path(&target_folder.path);
    let folder_relative_path = logical_path_to_relative_path(&logical_folder_path);
    let folder_absolute_path = user_root.join(&folder_relative_path);

    if fs::create_dir_all(&folder_absolute_path).is_err() {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::internal_with_context("Failed to create upload directory on the filesystem"),
        ));
    }

    let absolute_storage_path = folder_absolute_path.join(&file_name);
    if absolute_storage_path.exists() {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::Conflict(
                "An item with the same name already exists in this folder".to_owned(),
            ),
        ));
    }

    if let Err(error) = move_temp_file(&temp_file_path, &absolute_storage_path) {
        return Err(cleanup_temp_file_and_return(&temp_file_path, error));
    }

    let storage_rel_path = Path::new("users")
        .join(current_user.user.id.to_string())
        .join(folder_relative_path)
        .join(&file_name);
    let storage_path = format!("/{}", storage_rel_path.to_string_lossy().replace('\\', "/"));

    let mime_type = input
        .content_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned);

    let inserted = match sqlx::query_as::<_, StorageEntryRow>(
        r#"
        INSERT INTO files (
            owner_user_id,
            folder_id,
            name,
            original_file_name,
            mime_type,
            extension,
            size_bytes,
            storage_path,
            checksum,
            is_deleted
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, false)
        RETURNING
            name,
            CASE
                WHEN $10::TEXT = '/' THEN '/' || name
                ELSE $10::TEXT || '/' || name
            END AS path,
            'file'::TEXT AS entry_type,
            size_bytes,
            (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        "#,
    )
    .bind(current_user.user.id)
    .bind(target_folder.id)
    .bind(&file_name)
    .bind(&file_name)
    .bind(mime_type)
    .bind(extension)
    .bind(input.file_size_bytes)
    .bind(&storage_path)
    .bind(input.checksum)
    .bind(logical_folder_path)
    .fetch_one(&mut *tx)
    .await
    {
        Ok(entry) => entry,
        Err(error) => {
            let _ = fs::remove_file(&absolute_storage_path);
            return Err(map_storage_write_error(error));
        }
    };

    if let Err(error) = sqlx::query(
        r#"
        UPDATE users
        SET storage_used_bytes = storage_used_bytes + $2,
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(current_user.user.id)
    .bind(input.file_size_bytes)
    .execute(&mut *tx)
    .await
    {
        let _ = fs::remove_file(&absolute_storage_path);
        return Err(map_storage_write_error(error));
    }

    if tx.commit().await.is_err() {
        let _ = fs::remove_file(&absolute_storage_path);
        return Err(ApiError::internal_with_context(
            "Failed to commit file upload transaction",
        ));
    }

    Ok(storage_entry_row_to_dto(inserted))
}

pub async fn delete_file(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<DeletedStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Requested path must point to a file".to_owned(),
        ));
    }

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start file deletion transaction")
    })?;

    let target_file =
        load_file_for_deletion_tx(&mut tx, current_user.user.id, &requested_path).await?;

    let deleted = sqlx::query(
        r#"
        UPDATE files
        SET is_deleted = true,
            deleted_at = NOW(),
            updated_at = NOW()
        WHERE id = $1
          AND owner_user_id = $2
          AND is_deleted = false
        "#,
    )
    .bind(target_file.id)
    .bind(current_user.user.id)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Requested file does not exist".to_owned(),
        ));
    }

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit file deletion transaction")
    })?;

    Ok(DeletedStorageResult {
        deleted_path: normalize_db_path(&target_file.logical_path),
        entry_type: "file".to_owned(),
        reclaimed_bytes: 0,
    })
}

pub async fn delete_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<DeletedStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Root folder cannot be deleted".to_owned(),
        ));
    }

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start folder deletion transaction")
    })?;

    let target_folder =
        load_folder_for_deletion_tx(&mut tx, current_user.user.id, &requested_path).await?;

    let folder_prefix = format!("{}/%", target_folder.path.trim_end_matches('/'));
    let deleted_folders = sqlx::query(
        r#"
        UPDATE folders
        SET is_deleted = true,
            deleted_at = NOW(),
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND is_deleted = false
          AND (
              path = $2
              OR path LIKE $3
          )
        "#,
    )
    .bind(current_user.user.id)
    .bind(&target_folder.path)
    .bind(&folder_prefix)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    if deleted_folders.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Requested folder does not exist".to_owned(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE files
        SET is_deleted = true,
            deleted_at = NOW(),
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND is_deleted = false
          AND folder_id IN (
              SELECT id
              FROM folders
              WHERE owner_user_id = $1
                AND (
                    path = $2
                    OR path LIKE $3
                )
          )
        "#,
    )
    .bind(current_user.user.id)
    .bind(&target_folder.path)
    .bind(&folder_prefix)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit folder deletion transaction")
    })?;

    Ok(DeletedStorageResult {
        deleted_path: normalize_db_path(&target_folder.path),
        entry_type: "folder".to_owned(),
        reclaimed_bytes: 0,
    })
}

pub async fn permanently_delete_file(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<DeletedStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Requested path must point to a file".to_owned(),
        ));
    }

    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start permanent file deletion transaction")
    })?;

    let target_file =
        load_trashed_file_for_deletion_tx(&mut tx, current_user.user.id, &requested_path).await?;

    let deleted = sqlx::query(
        r#"
        DELETE FROM files
        WHERE id = $1
          AND owner_user_id = $2
          AND is_deleted = true
        "#,
    )
    .bind(target_file.id)
    .bind(current_user.user.id)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Requested file does not exist in trash".to_owned(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE users
        SET storage_used_bytes = GREATEST(0, storage_used_bytes - $2),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(current_user.user.id)
    .bind(target_file.size_bytes)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit permanent file deletion transaction")
    })?;

    let absolute_file_path = PathBuf::from(&settings.storage_root_path)
        .join(target_file.storage_path.trim_start_matches('/'));
    remove_file_if_exists(&absolute_file_path)?;

    Ok(DeletedStorageResult {
        deleted_path: normalize_db_path(&target_file.logical_path),
        entry_type: "file".to_owned(),
        reclaimed_bytes: target_file.size_bytes,
    })
}

pub async fn permanently_delete_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<DeletedStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Root folder cannot be deleted".to_owned(),
        ));
    }

    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start permanent folder deletion transaction")
    })?;

    let target_folder =
        load_trashed_folder_for_deletion_tx(&mut tx, current_user.user.id, &requested_path).await?;

    let reclaimed_bytes =
        load_folder_subtree_file_size_tx(&mut tx, current_user.user.id, &target_folder.path, true)
            .await?;

    let folder_prefix = format!("{}/%", target_folder.path.trim_end_matches('/'));
    let deleted = sqlx::query(
        r#"
        DELETE FROM folders
        WHERE owner_user_id = $1
          AND (
              path = $2
              OR path LIKE $3
          )
        "#,
    )
    .bind(current_user.user.id)
    .bind(&target_folder.path)
    .bind(&folder_prefix)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    if deleted.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Requested folder does not exist in trash".to_owned(),
        ));
    }

    sqlx::query(
        r#"
        UPDATE users
        SET storage_used_bytes = GREATEST(0, storage_used_bytes - $2),
            updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(current_user.user.id)
    .bind(reclaimed_bytes)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit permanent folder deletion transaction")
    })?;

    let user_root = resolve_user_storage_root(&settings.storage_root_path, current_user.user.id);
    let folder_absolute_path = user_root.join(logical_path_to_relative_path(&target_folder.path));
    remove_dir_if_exists(&folder_absolute_path)?;

    Ok(DeletedStorageResult {
        deleted_path: normalize_db_path(&target_folder.path),
        entry_type: "folder".to_owned(),
        reclaimed_bytes,
    })
}

pub async fn restore_file(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<RestoredStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Requested path must point to a file".to_owned(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to start file restore transaction"))?;

    let target_file =
        load_trashed_file_for_deletion_tx(&mut tx, current_user.user.id, &requested_path).await?;

    let restored = sqlx::query(
        r#"
        UPDATE files
        SET is_deleted = false,
            deleted_at = NULL,
            updated_at = NOW()
        WHERE id = $1
          AND owner_user_id = $2
          AND is_deleted = true
        "#,
    )
    .bind(target_file.id)
    .bind(current_user.user.id)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    if restored.rows_affected() == 0 {
        return Err(ApiError::BadRequest(
            "Requested file does not exist in trash".to_owned(),
        ));
    }

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit file restore transaction")
    })?;

    Ok(RestoredStorageResult {
        restored_path: normalize_db_path(&target_file.logical_path),
        entry_type: "file".to_owned(),
    })
}

pub async fn restore_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
) -> Result<RestoredStorageResult, ApiError> {
    let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
    if requested_path == "/" {
        return Err(ApiError::BadRequest(
            "Root folder cannot be restored".to_owned(),
        ));
    }

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start folder restore transaction")
    })?;

    let target_folder =
        load_trashed_folder_for_restore_tx(&mut tx, current_user.user.id, &requested_path).await?;

    if target_folder.parent_is_deleted == Some(true) {
        return Err(ApiError::BadRequest(
            "Parent folder is in trash. Restore the parent folder first.".to_owned(),
        ));
    }

    let folder_prefix = format!("{}/%", target_folder.path.trim_end_matches('/'));
    sqlx::query(
        r#"
        UPDATE folders
        SET is_deleted = false,
            deleted_at = NULL,
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND is_deleted = true
          AND (
              path = $2
              OR path LIKE $3
          )
        "#,
    )
    .bind(current_user.user.id)
    .bind(&target_folder.path)
    .bind(&folder_prefix)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    sqlx::query(
        r#"
        UPDATE files
        SET is_deleted = false,
            deleted_at = NULL,
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND is_deleted = true
          AND folder_id IN (
              SELECT id
              FROM folders
              WHERE owner_user_id = $1
                AND (
                    path = $2
                    OR path LIKE $3
                )
          )
        "#,
    )
    .bind(current_user.user.id)
    .bind(&target_folder.path)
    .bind(&folder_prefix)
    .execute(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit folder restore transaction")
    })?;

    Ok(RestoredStorageResult {
        restored_path: normalize_db_path(&target_folder.path),
        entry_type: "folder".to_owned(),
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

async fn load_total_storage_usage_tx(tx: &mut Transaction<'_, Postgres>) -> Result<i64, ApiError> {
    sqlx::query_scalar::<_, i64>("SELECT COALESCE(SUM(storage_used_bytes), 0)::BIGINT FROM users")
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to load total storage usage"))
}

async fn load_user_storage_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
) -> Result<UserStorageRow, ApiError> {
    sqlx::query_as::<_, UserStorageRow>(
        r#"
        SELECT storage_quota_bytes, storage_used_bytes
        FROM users
        WHERE id = $1
        FOR UPDATE
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load user storage quota"))?
    .ok_or_else(|| ApiError::Unauthorized("Invalid user session".to_owned()))
}

async fn load_requested_folder(
    pool: &PgPool,
    user_id: i64,
    requested_path: &str,
) -> Result<FolderRow, ApiError> {
    let root_folder_id = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT root_folder_id
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load user root folder"))?
    .flatten()
    .ok_or_else(|| ApiError::BadRequest("User root folder is not configured".to_owned()))?;

    if requested_path == "/" {
        return sqlx::query_as::<_, FolderRow>(
            r#"
            SELECT id, path
            FROM folders
            WHERE id = $1
              AND owner_user_id = $2
              AND is_deleted = false
            LIMIT 1
            "#,
        )
        .bind(root_folder_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to resolve user root folder"))?
        .ok_or_else(|| ApiError::BadRequest("User root folder was not found".to_owned()));
    }

    sqlx::query_as::<_, FolderRow>(
        r#"
        SELECT id, path
        FROM folders
        WHERE owner_user_id = $1
          AND path = $2
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve requested storage path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested storage path does not exist".to_owned()))
}

async fn load_requested_folder_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<FolderRow, ApiError> {
    let root_folder_id = sqlx::query_scalar::<_, Option<i64>>(
        r#"
        SELECT root_folder_id
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load user root folder"))?
    .flatten()
    .ok_or_else(|| ApiError::BadRequest("User root folder is not configured".to_owned()))?;

    if requested_path == "/" {
        return sqlx::query_as::<_, FolderRow>(
            r#"
            SELECT id, path
            FROM folders
            WHERE id = $1
              AND owner_user_id = $2
              AND is_deleted = false
            LIMIT 1
            "#,
        )
        .bind(root_folder_id)
        .bind(user_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to resolve user root folder"))?
        .ok_or_else(|| ApiError::BadRequest("User root folder was not found".to_owned()));
    }

    sqlx::query_as::<_, FolderRow>(
        r#"
        SELECT id, path
        FROM folders
        WHERE owner_user_id = $1
          AND path = $2
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve requested storage path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested storage path does not exist".to_owned()))
}

async fn load_file_for_deletion_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<DeleteFileRow, ApiError> {
    sqlx::query_as::<_, DeleteFileRow>(
        r#"
        SELECT
            file_row.id,
            file_row.size_bytes,
            file_row.storage_path,
            CASE
                WHEN folder.path = '/' THEN '/' || file_row.name
                ELSE folder.path || '/' || file_row.name
            END AS logical_path
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.owner_user_id = $1
          AND folder.owner_user_id = $1
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
          AND (
              CASE
                  WHEN folder.path = '/' THEN '/' || file_row.name
                  ELSE folder.path || '/' || file_row.name
              END
          ) = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve file deletion path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))
}

async fn load_trashed_file_for_deletion_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<DeleteFileRow, ApiError> {
    sqlx::query_as::<_, DeleteFileRow>(
        r#"
        SELECT
            file_row.id,
            file_row.size_bytes,
            file_row.storage_path,
            CASE
                WHEN folder.path = '/' THEN '/' || file_row.name
                ELSE folder.path || '/' || file_row.name
            END AS logical_path
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.owner_user_id = $1
          AND folder.owner_user_id = $1
          AND file_row.is_deleted = true
          AND (
              CASE
                  WHEN folder.path = '/' THEN '/' || file_row.name
                  ELSE folder.path || '/' || file_row.name
              END
          ) = $2
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve trashed file path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist in trash".to_owned()))
}

async fn load_folder_for_deletion_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<DeleteFolderRow, ApiError> {
    sqlx::query_as::<_, DeleteFolderRow>(
        r#"
        SELECT path
        FROM folders
        WHERE owner_user_id = $1
          AND path = $2
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve folder deletion path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist".to_owned()))
}

async fn load_trashed_folder_for_deletion_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<DeleteFolderRow, ApiError> {
    sqlx::query_as::<_, DeleteFolderRow>(
        r#"
        SELECT path
        FROM folders
        WHERE owner_user_id = $1
          AND path = $2
          AND is_deleted = true
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve trashed folder path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist in trash".to_owned()))
}

async fn load_trashed_folder_for_restore_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<TrashedFolderRestoreRow, ApiError> {
    sqlx::query_as::<_, TrashedFolderRestoreRow>(
        r#"
        SELECT
            folder.path,
            parent.is_deleted AS parent_is_deleted
        FROM folders folder
        LEFT JOIN folders parent
            ON parent.id = folder.parent_folder_id
        WHERE folder.owner_user_id = $1
          AND folder.path = $2
          AND folder.is_deleted = true
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .bind(requested_path)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve trashed folder path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist in trash".to_owned()))
}

async fn load_folder_subtree_file_size_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    folder_path: &str,
    is_deleted: bool,
) -> Result<i64, ApiError> {
    let prefix = format!("{}/%", folder_path.trim_end_matches('/'));

    sqlx::query_scalar::<_, i64>(
        r#"
        SELECT COALESCE(SUM(file_row.size_bytes), 0)::BIGINT
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.owner_user_id = $1
          AND folder.owner_user_id = $1
          AND file_row.is_deleted = $4
          AND (
              folder.path = $2
              OR folder.path LIKE $3
          )
        "#,
    )
    .bind(user_id)
    .bind(folder_path)
    .bind(prefix)
    .bind(is_deleted)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to calculate folder reclaim size"))
}

async fn ensure_name_not_taken_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_user_id: i64,
    parent_folder_id: i64,
    name: &str,
) -> Result<(), ApiError> {
    let name_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM folders
            WHERE owner_user_id = $1
              AND parent_folder_id = $2
              AND name = $3
              AND is_deleted = false
        )
        OR EXISTS (
            SELECT 1
            FROM files
            WHERE owner_user_id = $1
              AND folder_id = $2
              AND name = $3
              AND is_deleted = false
        )
        "#,
    )
    .bind(owner_user_id)
    .bind(parent_folder_id)
    .bind(name)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to validate duplicate storage names"))?;

    if name_exists {
        return Err(ApiError::Conflict(
            "An item with the same name already exists in this folder".to_owned(),
        ));
    }

    Ok(())
}

async fn load_storage_entries(
    pool: &PgPool,
    user_id: i64,
    parent_folder_id: i64,
    parent_folder_path: &str,
    search: Option<&str>,
) -> Result<Vec<StorageEntryRow>, ApiError> {
    sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            entries.name,
            entries.path,
            entries.entry_type,
            entries.size_bytes,
            entries.modified_at_unix_ms
        FROM (
            SELECT
                f.name,
                f.path,
                'folder'::TEXT AS entry_type,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM f.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM folders f
            WHERE f.owner_user_id = $1
              AND f.parent_folder_id = $2
              AND f.is_deleted = false
              AND ($3::TEXT IS NULL OR f.name ILIKE '%' || $3 || '%')

            UNION ALL

            SELECT
                file_row.name,
                CASE
                    WHEN $4::TEXT = '/' THEN '/' || file_row.name
                    ELSE $4::TEXT || '/' || file_row.name
                END AS path,
                'file'::TEXT AS entry_type,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM files file_row
            WHERE file_row.owner_user_id = $1
              AND file_row.folder_id = $2
              AND file_row.is_deleted = false
              AND ($3::TEXT IS NULL OR file_row.name ILIKE '%' || $3 || '%')
        ) entries
        ORDER BY
            CASE WHEN entries.entry_type = 'folder' THEN 0 ELSE 1 END,
            LOWER(entries.name),
            entries.name
        "#,
    )
    .bind(user_id)
    .bind(parent_folder_id)
    .bind(search)
    .bind(parent_folder_path)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load storage listing"))
}

async fn load_trashed_storage_entries(
    pool: &PgPool,
    user_id: i64,
    search: Option<&str>,
) -> Result<Vec<StorageEntryRow>, ApiError> {
    sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            entries.name,
            entries.path,
            entries.entry_type,
            entries.size_bytes,
            entries.modified_at_unix_ms
        FROM (
            SELECT
                folder.name,
                folder.path,
                'folder'::TEXT AS entry_type,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM COALESCE(folder.deleted_at, folder.updated_at)) * 1000)::BIGINT AS modified_at_unix_ms
            FROM folders folder
            LEFT JOIN folders parent
                ON parent.id = folder.parent_folder_id
            WHERE folder.owner_user_id = $1
              AND folder.is_deleted = true
              AND (
                  parent.id IS NULL
                  OR parent.is_deleted = false
              )
              AND ($2::TEXT IS NULL OR folder.name ILIKE '%' || $2 || '%')

            UNION ALL

            SELECT
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                'file'::TEXT AS entry_type,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM COALESCE(file_row.deleted_at, file_row.updated_at)) * 1000)::BIGINT AS modified_at_unix_ms
            FROM files file_row
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            WHERE file_row.owner_user_id = $1
              AND folder.owner_user_id = $1
              AND file_row.is_deleted = true
              AND folder.is_deleted = false
              AND ($2::TEXT IS NULL OR file_row.name ILIKE '%' || $2 || '%')
        ) entries
        ORDER BY entries.modified_at_unix_ms DESC, LOWER(entries.name), entries.name
        "#,
    )
    .bind(user_id)
    .bind(search)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load trash listing"))
}

fn storage_entry_row_to_dto(row: StorageEntryRow) -> StorageEntryDto {
    StorageEntryDto {
        name: row.name,
        path: normalize_db_path(&row.path),
        entry_type: row.entry_type,
        size_bytes: row.size_bytes,
        modified_at_unix_ms: Some(row.modified_at_unix_ms),
    }
}

fn normalize_api_path(raw: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() || trimmed == "/" {
        return Ok("/".to_owned());
    }

    let mut segments = Vec::new();

    for segment in trimmed.split('/') {
        let clean = segment.trim();

        if clean.is_empty() || clean == "." {
            continue;
        }

        if clean == ".." {
            return Err(ApiError::BadRequest(
                "Storage path contains invalid path segments".to_owned(),
            ));
        }

        if clean.contains('\\') {
            return Err(ApiError::BadRequest(
                "Storage path contains invalid path separators".to_owned(),
            ));
        }

        segments.push(clean);
    }

    if segments.is_empty() {
        Ok("/".to_owned())
    } else {
        Ok(format!("/{}", segments.join("/")))
    }
}

fn normalize_db_path(value: &str) -> String {
    let trimmed = value.trim();

    if trimmed.is_empty() {
        "/".to_owned()
    } else if trimmed.starts_with('/') {
        trimmed.to_owned()
    } else {
        format!("/{trimmed}")
    }
}

fn parent_api_path(current_path: &str) -> Option<String> {
    let normalized = normalize_db_path(current_path);

    if normalized == "/" {
        return None;
    }

    let mut segments: Vec<&str> = normalized
        .split('/')
        .filter(|value| !value.is_empty())
        .collect();
    segments.pop();

    if segments.is_empty() {
        Some("/".to_owned())
    } else {
        Some(format!("/{}", segments.join("/")))
    }
}

fn normalize_item_name(raw: &str, field_name: &str) -> Result<String, ApiError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(ApiError::BadRequest(format!(
            "{field_name} cannot be empty"
        )));
    }

    if trimmed == "." || trimmed == ".." {
        return Err(ApiError::BadRequest(format!(
            "{field_name} cannot be '.' or '..'"
        )));
    }

    if trimmed.contains('/') || trimmed.contains('\\') {
        return Err(ApiError::BadRequest(format!(
            "{field_name} cannot contain path separators"
        )));
    }

    if trimmed.chars().any(char::is_control) {
        return Err(ApiError::BadRequest(format!(
            "{field_name} contains invalid characters"
        )));
    }

    if trimmed.len() > 255 {
        return Err(ApiError::BadRequest(format!(
            "{field_name} is too long (maximum is 255 characters)"
        )));
    }

    Ok(trimmed.to_owned())
}

fn normalize_file_name(raw: &str) -> Result<String, ApiError> {
    let base_name = Path::new(raw)
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| ApiError::BadRequest("Uploaded file name is invalid".to_owned()))?;

    normalize_item_name(base_name, "File name")
}

fn extract_extension(file_name: &str) -> Option<String> {
    Path::new(file_name)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_lowercase)
}

fn join_child_path(parent_path: &str, child_name: &str) -> String {
    if parent_path == "/" {
        format!("/{child_name}")
    } else {
        format!("{}/{}", normalize_db_path(parent_path), child_name)
    }
}

fn resolve_user_storage_root(storage_root_path: &str, user_id: i64) -> PathBuf {
    PathBuf::from(storage_root_path)
        .join("users")
        .join(user_id.to_string())
}

fn logical_path_to_relative_path(logical_path: &str) -> PathBuf {
    let normalized = normalize_db_path(logical_path);
    let mut relative = PathBuf::new();

    for segment in normalized.split('/') {
        if segment.is_empty() {
            continue;
        }

        relative.push(segment);
    }

    relative
}

fn move_temp_file(source: &Path, target: &Path) -> Result<(), ApiError> {
    match fs::rename(source, target) {
        Ok(_) => Ok(()),
        Err(_) => {
            fs::copy(source, target).map_err(|_| {
                ApiError::internal_with_context("Failed to move uploaded file to final destination")
            })?;
            fs::remove_file(source).map_err(|_| {
                ApiError::internal_with_context("Failed to clean up temporary uploaded file")
            })?;
            Ok(())
        }
    }
}

fn remove_file_if_exists(path: &Path) -> Result<(), ApiError> {
    match fs::remove_file(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(ApiError::internal_with_context(
            "Failed to delete file from storage filesystem",
        )),
    }
}

fn remove_dir_if_exists(path: &Path) -> Result<(), ApiError> {
    match fs::remove_dir_all(path) {
        Ok(_) => Ok(()),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(ApiError::internal_with_context(
            "Failed to delete folder from storage filesystem",
        )),
    }
}

fn cleanup_temp_file_and_return(temp_file_path: &Path, error: ApiError) -> ApiError {
    let _ = fs::remove_file(temp_file_path);
    error
}

fn map_storage_write_error(error: sqlx::Error) -> ApiError {
    match &error {
        sqlx::Error::Database(db_error) if db_error.code().as_deref() == Some("23505") => {
            ApiError::Conflict(
                "An item with the same name already exists in this folder".to_owned(),
            )
        }
        _ => ApiError::internal_with_context("Database error during storage operation"),
    }
}
