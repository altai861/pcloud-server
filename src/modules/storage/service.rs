use crate::{
    error::ApiError,
    modules::{auth::service::AuthenticatedUser, storage::dto::StorageEntryDto},
};
use rand_core::{OsRng, RngCore};
use sqlx::{FromRow, PgPool, Postgres, Transaction};
use std::{
    collections::HashSet,
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use zip::{CompressionMethod, ZipWriter, write::FileOptions};

const MAX_UPLOAD_SIZE_BYTES: i64 = 5 * 1024 * 1024 * 1024;
const DEFAULT_STORAGE_LIST_LIMIT: i64 = 200;
const MAX_STORAGE_LIST_LIMIT: i64 = 500;
const DEFAULT_SEARCH_LIST_LIMIT: i64 = 120;
const MAX_SEARCH_LIST_LIMIT: i64 = 300;

#[derive(Debug, Clone)]
pub struct StorageListQuery {
    pub path: Option<String>,
    pub folder_id: Option<i64>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateFolderInput {
    pub parent_path: Option<String>,
    pub parent_folder_id: Option<i64>,
    pub name: String,
}

#[derive(Debug)]
pub struct UploadFileInput {
    pub folder_path: Option<String>,
    pub folder_id: Option<i64>,
    pub file_name: String,
    pub content_type: Option<String>,
    pub temp_file_path: PathBuf,
    pub file_size_bytes: i64,
    pub checksum: String,
}

#[derive(Debug, Clone)]
pub struct DownloadFileQuery {
    pub path: Option<String>,
    pub file_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct BatchDownloadItemInput {
    pub resource_type: StorageEntryKind,
    pub resource_id: i64,
}

#[derive(Debug, Clone)]
pub struct MoveStorageInput {
    pub destination_folder_id: i64,
    pub items: Vec<MoveStorageItemInput>,
}

#[derive(Debug, Clone)]
pub struct MoveStorageItemInput {
    pub resource_type: StorageEntryKind,
    pub resource_id: i64,
}

#[derive(Debug, Clone)]
pub struct RenameStorageInput {
    pub path: Option<String>,
    pub resource_id: Option<i64>,
    pub new_name: String,
}

#[derive(Debug, Clone)]
pub struct SharePermissionInput {
    pub resource_type: StorageEntryKind,
    pub resource_id: i64,
    pub target_user_id: i64,
    pub privilege_type: String,
}

#[derive(Debug, Clone)]
pub struct RemoveSharePermissionInput {
    pub resource_type: StorageEntryKind,
    pub resource_id: i64,
    pub target_user_id: i64,
}

#[derive(Debug, Clone)]
pub struct SharePermissionsQuery {
    pub resource_type: StorageEntryKind,
    pub resource_id: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageEntryKind {
    Folder,
    File,
}

#[derive(Debug, Clone)]
pub struct SetStarredInput {
    pub path: Option<String>,
    pub entry_type: StorageEntryKind,
    pub starred: bool,
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
    parent_folder_id: Option<i64>,
    owner_user_id: i64,
}

#[derive(Debug, FromRow)]
struct StorageEntryRow {
    id: i64,
    name: String,
    path: String,
    entry_type: String,
    owner_user_id: i64,
    owner_username: String,
    created_by_user_id: Option<i64>,
    created_by_username: String,
    is_starred: bool,
    size_bytes: Option<i64>,
    modified_at_unix_ms: i64,
}

#[derive(Debug, FromRow)]
struct FolderMetadataRow {
    name: String,
    path: String,
    owner_username: String,
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
struct BatchFileDownloadRow {
    storage_path: String,
    logical_path: String,
}

#[derive(Debug, FromRow)]
struct FileMetadataRow {
    id: i64,
    folder_id: i64,
    folder_path: String,
    owner_user_id: i64,
    owner_username: String,
    name: String,
    path: String,
    size_bytes: i64,
    mime_type: Option<String>,
    extension: Option<String>,
    is_starred: bool,
    access_rank: i32,
    created_at_unix_ms: i64,
    modified_at_unix_ms: i64,
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

#[derive(Debug, FromRow)]
struct RenameFileRow {
    id: i64,
    owner_user_id: i64,
    created_by_user_id: Option<i64>,
    name: String,
    is_starred: bool,
    storage_path: String,
    folder_id: i64,
    folder_path: String,
}

#[derive(Debug, FromRow)]
struct RenameFileAccessRow {
    id: i64,
    owner_user_id: i64,
    created_by_user_id: Option<i64>,
    name: String,
    is_starred: bool,
    storage_path: String,
    folder_id: i64,
    folder_path: String,
    access_rank: i32,
}

#[derive(Debug, FromRow)]
struct RenameFolderRow {
    id: i64,
    owner_user_id: i64,
    created_by_user_id: Option<i64>,
    name: String,
    is_starred: bool,
    path: String,
    parent_folder_id: Option<i64>,
}

#[derive(Debug, FromRow)]
struct MoveFolderRow {
    id: i64,
    owner_user_id: i64,
    name: String,
    path: String,
    parent_folder_id: Option<i64>,
}

#[derive(Debug, FromRow)]
struct MoveFileRow {
    id: i64,
    owner_user_id: i64,
    name: String,
    storage_path: String,
    folder_id: i64,
    folder_path: String,
}

#[derive(Debug, FromRow)]
struct SharedResourceRow {
    resource_type: String,
    resource_id: i64,
    name: String,
    path: String,
    owner_user_id: i64,
    owner_username: String,
    created_by_user_id: Option<i64>,
    created_by_username: String,
    privilege_type: String,
    date_shared_unix_ms: i64,
}

#[derive(Debug, FromRow)]
struct SearchResourceRow {
    resource_type: String,
    resource_id: i64,
    name: String,
    path: String,
    owner_user_id: i64,
    owner_username: String,
    created_by_user_id: Option<i64>,
    created_by_username: String,
    source_context: String,
    navigate_folder_id: i64,
    size_bytes: Option<i64>,
    modified_at_unix_ms: i64,
    access_rank: i32,
}

#[derive(Debug, FromRow)]
struct SharedPermissionRow {
    user_id: i64,
    username: String,
    full_name: String,
    privilege_type: String,
    created_at_unix_ms: i64,
}

#[derive(Debug, FromRow)]
struct ShareableUserRow {
    user_id: i64,
    username: String,
    full_name: String,
}

#[derive(Debug, FromRow)]
struct ResourceOwnerRow {
    owner_user_id: i64,
    resource_name: String,
}

#[derive(Debug, FromRow)]
struct FolderAccessRow {
    id: i64,
    path: String,
    parent_folder_id: Option<i64>,
    owner_user_id: i64,
    access_rank: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessLevel {
    Owner,
    Editor,
    Viewer,
}

impl AccessLevel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Editor => "editor",
            Self::Viewer => "viewer",
        }
    }

    fn is_editor(self) -> bool {
        matches!(self, Self::Owner | Self::Editor)
    }
}

#[derive(Debug)]
pub struct StorageListResult {
    pub current_path: String,
    pub current_folder_id: Option<i64>,
    pub parent_folder_id: Option<i64>,
    pub parent_path: Option<String>,
    pub current_privilege: String,
    pub entries: Vec<StorageEntryDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
    pub total_storage_limit_bytes: Option<i64>,
    pub total_storage_used_bytes: i64,
    pub user_storage_quota_bytes: i64,
    pub user_storage_used_bytes: i64,
}

#[derive(Debug)]
pub struct StorageFolderMetadataResult {
    pub name: String,
    pub path: String,
    pub owner_username: String,
    pub current_privilege: String,
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
pub struct BatchDownloadResult {
    pub archive_name: String,
    pub archive_path: PathBuf,
    pub archive_size_bytes: u64,
}

#[derive(Debug)]
pub struct MoveStorageResult {
    pub moved_count: i64,
    pub destination_folder_id: i64,
    pub destination_path: String,
}

#[derive(Debug)]
pub struct StorageFileMetadataResult {
    pub id: i64,
    pub folder_id: i64,
    pub folder_path: String,
    pub owner_user_id: i64,
    pub owner_username: String,
    pub current_privilege: String,
    pub name: String,
    pub path: String,
    pub size_bytes: i64,
    pub mime_type: Option<String>,
    pub extension: Option<String>,
    pub is_starred: bool,
    pub created_at_unix_ms: i64,
    pub modified_at_unix_ms: i64,
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

#[derive(Debug)]
pub struct SharedResourceEntryResult {
    pub resource_type: String,
    pub resource_id: i64,
    pub name: String,
    pub path: String,
    pub owner_user_id: i64,
    pub owner_username: String,
    pub created_by_user_id: Option<i64>,
    pub created_by_username: String,
    pub privilege_type: String,
    pub date_shared_unix_ms: i64,
}

#[derive(Debug)]
pub struct SearchResourceEntryResult {
    pub resource_type: String,
    pub resource_id: i64,
    pub name: String,
    pub path: String,
    pub owner_user_id: i64,
    pub owner_username: String,
    pub created_by_user_id: Option<i64>,
    pub created_by_username: String,
    pub source_context: String,
    pub privilege_type: String,
    pub navigate_folder_id: i64,
    pub size_bytes: Option<i64>,
    pub modified_at_unix_ms: i64,
}

#[derive(Debug)]
pub struct SearchResourcesResult {
    pub query: String,
    pub entries: Vec<SearchResourceEntryResult>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug)]
pub struct SharedPermissionsResult {
    pub resource_type: String,
    pub resource_id: i64,
    pub resource_name: String,
    pub entries: Vec<SharedPermissionTargetResult>,
}

#[derive(Debug)]
pub struct SharedPermissionTargetResult {
    pub user_id: i64,
    pub username: String,
    pub full_name: String,
    pub privilege_type: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug)]
pub struct ShareableUserResult {
    pub user_id: i64,
    pub username: String,
    pub full_name: String,
}

pub async fn list_user_storage(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: StorageListQuery,
) -> Result<StorageListResult, ApiError> {
    let settings = load_system_storage_settings(pool).await?;
    let total_storage_used_bytes = load_total_storage_usage(pool).await?;

    let (current_folder, current_access) = if let Some(folder_id) = query.folder_id {
        load_accessible_folder_by_id(pool, current_user.user.id, folder_id).await?
    } else {
        let requested_path = normalize_api_path(query.path.as_deref().unwrap_or("/"))?;
        (
            load_requested_folder(pool, current_user.user.id, &requested_path).await?,
            AccessLevel::Owner,
        )
    };

    let search = query
        .search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let page_limit = normalize_storage_list_limit(query.limit);
    let page_offset = parse_storage_list_cursor(query.cursor.as_deref())?;

    let (storage_entries, next_cursor, has_more) = load_storage_entries(
        pool,
        current_folder.id,
        &current_folder.path,
        search,
        page_limit,
        page_offset,
    )
    .await?;

    let entries = storage_entries
        .into_iter()
        .map(|entry| StorageEntryDto {
            id: entry.id,
            name: entry.name,
            path: normalize_db_path(&entry.path),
            entry_type: entry.entry_type,
            owner_user_id: entry.owner_user_id,
            owner_username: entry.owner_username,
            created_by_user_id: entry.created_by_user_id,
            created_by_username: entry.created_by_username,
            is_starred: entry.is_starred,
            size_bytes: entry.size_bytes,
            modified_at_unix_ms: Some(entry.modified_at_unix_ms),
        })
        .collect();

    let current_path = normalize_db_path(&current_folder.path);

    Ok(StorageListResult {
        current_path: current_path.clone(),
        current_folder_id: Some(current_folder.id),
        parent_folder_id: current_folder.parent_folder_id,
        parent_path: parent_api_path(&current_path),
        current_privilege: current_access.as_str().to_owned(),
        entries,
        next_cursor,
        has_more,
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
        current_folder_id: None,
        parent_folder_id: None,
        parent_path: None,
        current_privilege: "owner".to_owned(),
        entries,
        next_cursor: None,
        has_more: false,
        total_storage_limit_bytes: settings.total_storage_limit_bytes,
        total_storage_used_bytes,
        user_storage_quota_bytes: current_user.user.storage_quota_bytes,
        user_storage_used_bytes: current_user.user.storage_used_bytes,
    })
}

pub async fn list_user_starred(
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

    let entries = load_starred_storage_entries(pool, current_user.user.id, search)
        .await?
        .into_iter()
        .map(storage_entry_row_to_dto)
        .collect();

    Ok(StorageListResult {
        current_path: "/starred".to_owned(),
        current_folder_id: None,
        parent_folder_id: None,
        parent_path: None,
        current_privilege: "owner".to_owned(),
        entries,
        next_cursor: None,
        has_more: false,
        total_storage_limit_bytes: settings.total_storage_limit_bytes,
        total_storage_used_bytes,
        user_storage_quota_bytes: current_user.user.storage_quota_bytes,
        user_storage_used_bytes: current_user.user.storage_used_bytes,
    })
}

pub async fn list_shared_with_user(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    search: Option<String>,
) -> Result<Vec<SharedResourceEntryResult>, ApiError> {
    let search = search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let rows = sqlx::query_as::<_, SharedResourceRow>(
        r#"
        SELECT
            entries.resource_type,
            entries.resource_id,
            entries.name,
            entries.path,
            entries.owner_user_id,
            entries.owner_username,
            entries.created_by_user_id,
            entries.created_by_username,
            entries.privilege_type,
            entries.date_shared_unix_ms
        FROM (
            SELECT
                'folder'::TEXT AS resource_type,
                folder.id AS resource_id,
                folder.name,
                folder.path,
                folder.owner_user_id,
                owner_user.username AS owner_username,
                folder.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                CASE
                    WHEN BOOL_OR(lower(folder_perm.privilege_type) IN ('editor', 'edit')) THEN 'editor'
                    ELSE 'viewer'
                END AS privilege_type,
                (EXTRACT(EPOCH FROM MAX(folder_perm.created_at)) * 1000)::BIGINT AS date_shared_unix_ms
            FROM folder_permissions folder_perm
            INNER JOIN folders folder
                ON folder.id = folder_perm.folder_id
            INNER JOIN users owner_user
                ON owner_user.id = folder.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = folder.created_by_user_id
            WHERE folder_perm.user_id = $1
              AND folder.owner_user_id <> $1
              AND folder.is_deleted = false
              AND (
                  $2::TEXT IS NULL
                  OR folder.name ILIKE '%' || $2 || '%'
                  OR owner_user.username ILIKE '%' || $2 || '%'
              )
            GROUP BY
                folder.id,
                folder.name,
                folder.path,
                folder.owner_user_id,
                owner_user.username,
                folder.created_by_user_id,
                creator_user.username

            UNION ALL

            SELECT
                'file'::TEXT AS resource_type,
                file_row.id AS resource_id,
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                CASE
                    WHEN BOOL_OR(lower(file_perm.privilege_type) IN ('editor', 'edit')) THEN 'editor'
                    ELSE 'viewer'
                END AS privilege_type,
                (EXTRACT(EPOCH FROM MAX(file_perm.created_at)) * 1000)::BIGINT AS date_shared_unix_ms
            FROM file_permissions file_perm
            INNER JOIN files file_row
                ON file_row.id = file_perm.file_id
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
            WHERE file_perm.user_id = $1
              AND file_row.owner_user_id <> $1
              AND file_row.is_deleted = false
              AND folder.is_deleted = false
              AND (
                  $2::TEXT IS NULL
                  OR file_row.name ILIKE '%' || $2 || '%'
                  OR owner_user.username ILIKE '%' || $2 || '%'
              )
            GROUP BY
                file_row.id,
                file_row.name,
                folder.path,
                file_row.owner_user_id,
                owner_user.username,
                file_row.created_by_user_id,
                creator_user.username
        ) entries
        ORDER BY entries.date_shared_unix_ms DESC, LOWER(entries.name), entries.name
        "#,
    )
    .bind(current_user.user.id)
    .bind(search)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load shared resources listing"))?;

    let entries: Vec<SharedResourceEntryResult> = rows
        .into_iter()
        .map(|row| SharedResourceEntryResult {
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            name: row.name,
            path: normalize_db_path(&row.path),
            owner_user_id: row.owner_user_id,
            owner_username: row.owner_username,
            created_by_user_id: row.created_by_user_id,
            created_by_username: row.created_by_username,
            privilege_type: row.privilege_type,
            date_shared_unix_ms: row.date_shared_unix_ms,
        })
        .collect();

    let shared_folder_roots: Vec<(i64, String)> = entries
        .iter()
        .filter(|entry| entry.resource_type == "folder")
        .map(|entry| (entry.owner_user_id, entry.path.clone()))
        .collect();

    Ok(entries
        .into_iter()
        .filter(|entry| {
            if entry.resource_type == "folder" {
                return !shared_folder_roots.iter().any(|(owner_id, folder_path)| {
                    *owner_id == entry.owner_user_id
                        && *folder_path != entry.path
                        && is_descendant_path(&entry.path, folder_path)
                });
            }

            !shared_folder_roots.iter().any(|(owner_id, folder_path)| {
                *owner_id == entry.owner_user_id && is_descendant_path(&entry.path, folder_path)
            })
        })
        .collect())
}

pub async fn list_resource_permissions(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: SharePermissionsQuery,
) -> Result<SharedPermissionsResult, ApiError> {
    let owner = load_resource_owner(pool, query.resource_type, query.resource_id).await?;
    if owner.owner_user_id != current_user.user.id {
        return Err(ApiError::BadRequest(
            "Only the resource owner can manage sharing".to_owned(),
        ));
    }

    let rows = match query.resource_type {
        StorageEntryKind::Folder => {
            sqlx::query_as::<_, SharedPermissionRow>(
                r#"
                SELECT
                    target_user.id AS user_id,
                    target_user.username,
                    target_user.full_name,
                    CASE
                        WHEN BOOL_OR(lower(folder_perm.privilege_type) IN ('editor', 'edit')) THEN 'editor'
                        ELSE 'viewer'
                    END AS privilege_type,
                    (EXTRACT(EPOCH FROM MAX(folder_perm.created_at)) * 1000)::BIGINT AS created_at_unix_ms
                FROM folder_permissions folder_perm
                INNER JOIN users target_user
                    ON target_user.id = folder_perm.user_id
                WHERE folder_perm.folder_id = $1
                GROUP BY target_user.id, target_user.username, target_user.full_name
                ORDER BY LOWER(target_user.username), target_user.username
                "#,
            )
            .bind(query.resource_id)
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal_with_context("Failed to load folder permissions"))?
        }
        StorageEntryKind::File => {
            sqlx::query_as::<_, SharedPermissionRow>(
                r#"
                SELECT
                    target_user.id AS user_id,
                    target_user.username,
                    target_user.full_name,
                    CASE
                        WHEN BOOL_OR(lower(file_perm.privilege_type) IN ('editor', 'edit')) THEN 'editor'
                        ELSE 'viewer'
                    END AS privilege_type,
                    (EXTRACT(EPOCH FROM MAX(file_perm.created_at)) * 1000)::BIGINT AS created_at_unix_ms
                FROM file_permissions file_perm
                INNER JOIN users target_user
                    ON target_user.id = file_perm.user_id
                WHERE file_perm.file_id = $1
                GROUP BY target_user.id, target_user.username, target_user.full_name
                ORDER BY LOWER(target_user.username), target_user.username
                "#,
            )
            .bind(query.resource_id)
            .fetch_all(pool)
            .await
            .map_err(|_| ApiError::internal_with_context("Failed to load file permissions"))?
        }
    };

    Ok(SharedPermissionsResult {
        resource_type: match query.resource_type {
            StorageEntryKind::Folder => "folder".to_owned(),
            StorageEntryKind::File => "file".to_owned(),
        },
        resource_id: query.resource_id,
        resource_name: owner.resource_name,
        entries: rows
            .into_iter()
            .map(|row| SharedPermissionTargetResult {
                user_id: row.user_id,
                username: row.username,
                full_name: row.full_name,
                privilege_type: row.privilege_type,
                created_at_unix_ms: row.created_at_unix_ms,
            })
            .collect(),
    })
}

pub async fn upsert_share_permission(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: SharePermissionInput,
) -> Result<(), ApiError> {
    let privilege = normalize_share_privilege(&input.privilege_type)?;
    if input.target_user_id == current_user.user.id {
        return Err(ApiError::BadRequest(
            "You cannot grant permissions to yourself".to_owned(),
        ));
    }

    let owner = load_resource_owner(pool, input.resource_type, input.resource_id).await?;
    if owner.owner_user_id != current_user.user.id {
        return Err(ApiError::BadRequest(
            "Only the resource owner can manage sharing".to_owned(),
        ));
    }

    let target_user_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM users
            WHERE id = $1
              AND status = 'active'
        )
        "#,
    )
    .bind(input.target_user_id)
    .fetch_one(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to validate target user"))?;

    if !target_user_exists {
        return Err(ApiError::BadRequest(
            "Target user does not exist".to_owned(),
        ));
    }

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to start share update transaction"))?;

    match input.resource_type {
        StorageEntryKind::Folder => {
            sqlx::query(
                r#"
                DELETE FROM folder_permissions
                WHERE folder_id = $1
                  AND user_id = $2
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_storage_write_error)?;

            sqlx::query(
                r#"
                INSERT INTO folder_permissions (folder_id, user_id, privilege_type, granted_by_user_id)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .bind(privilege)
            .bind(current_user.user.id)
            .execute(&mut *tx)
            .await
            .map_err(map_storage_write_error)?;
        }
        StorageEntryKind::File => {
            sqlx::query(
                r#"
                DELETE FROM file_permissions
                WHERE file_id = $1
                  AND user_id = $2
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .execute(&mut *tx)
            .await
            .map_err(map_storage_write_error)?;

            sqlx::query(
                r#"
                INSERT INTO file_permissions (file_id, user_id, privilege_type, granted_by_user_id)
                VALUES ($1, $2, $3, $4)
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .bind(privilege)
            .bind(current_user.user.id)
            .execute(&mut *tx)
            .await
            .map_err(map_storage_write_error)?;
        }
    }

    tx.commit().await.map_err(|_| {
        ApiError::internal_with_context("Failed to commit share update transaction")
    })?;

    Ok(())
}

pub async fn remove_share_permission(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: RemoveSharePermissionInput,
) -> Result<(), ApiError> {
    if input.target_user_id == current_user.user.id {
        return Err(ApiError::BadRequest(
            "You cannot remove your own owner permissions".to_owned(),
        ));
    }

    let owner = load_resource_owner(pool, input.resource_type, input.resource_id).await?;
    if owner.owner_user_id != current_user.user.id {
        return Err(ApiError::BadRequest(
            "Only the resource owner can manage sharing".to_owned(),
        ));
    }

    match input.resource_type {
        StorageEntryKind::Folder => {
            sqlx::query(
                r#"
                DELETE FROM folder_permissions
                WHERE folder_id = $1
                  AND user_id = $2
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .execute(pool)
            .await
            .map_err(map_storage_write_error)?;
        }
        StorageEntryKind::File => {
            sqlx::query(
                r#"
                DELETE FROM file_permissions
                WHERE file_id = $1
                  AND user_id = $2
                "#,
            )
            .bind(input.resource_id)
            .bind(input.target_user_id)
            .execute(pool)
            .await
            .map_err(map_storage_write_error)?;
        }
    }

    Ok(())
}

pub async fn search_resources(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: Option<String>,
    limit: Option<i64>,
    cursor: Option<String>,
) -> Result<SearchResourcesResult, ApiError> {
    let normalized_query = query
        .unwrap_or_default()
        .trim()
        .to_owned();

    if normalized_query.is_empty() {
        return Ok(SearchResourcesResult {
            query: String::new(),
            entries: Vec::new(),
            next_cursor: None,
            has_more: false,
        });
    }

    let search_pattern = format!("%{normalized_query}%");
    let page_limit = normalize_search_list_limit(limit);
    let page_offset = parse_storage_list_cursor(cursor.as_deref())?;

    let (rows, next_cursor, has_more) = load_search_entries(
        pool,
        current_user.user.id,
        &search_pattern,
        page_limit,
        page_offset,
    )
    .await?;

    let entries = rows
        .into_iter()
        .map(|row| SearchResourceEntryResult {
            resource_type: row.resource_type,
            resource_id: row.resource_id,
            name: row.name,
            path: normalize_db_path(&row.path),
            owner_user_id: row.owner_user_id,
            owner_username: row.owner_username,
            created_by_user_id: row.created_by_user_id,
            created_by_username: row.created_by_username,
            source_context: row.source_context,
            privilege_type: access_level_from_rank(row.access_rank)
                .map(|level| level.as_str().to_owned())
                .unwrap_or_else(|| "viewer".to_owned()),
            navigate_folder_id: row.navigate_folder_id,
            size_bytes: row.size_bytes,
            modified_at_unix_ms: row.modified_at_unix_ms,
        })
        .collect();

    Ok(SearchResourcesResult {
        query: normalized_query,
        entries,
        next_cursor,
        has_more,
    })
}

pub async fn search_shareable_users(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    search: Option<String>,
) -> Result<Vec<ShareableUserResult>, ApiError> {
    let search = search
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());

    let rows = sqlx::query_as::<_, ShareableUserRow>(
        r#"
        SELECT
            user_row.id AS user_id,
            user_row.username,
            user_row.full_name
        FROM users user_row
        WHERE user_row.status = 'active'
          AND user_row.id <> $1
          AND (
              $2::TEXT IS NULL
              OR user_row.username ILIKE '%' || $2 || '%'
              OR user_row.full_name ILIKE '%' || $2 || '%'
          )
        ORDER BY LOWER(user_row.username), user_row.username
        LIMIT 20
        "#,
    )
    .bind(current_user.user.id)
    .bind(search)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to search users for sharing"))?;

    Ok(rows
        .into_iter()
        .map(|row| ShareableUserResult {
            user_id: row.user_id,
            username: row.username,
            full_name: row.full_name,
        })
        .collect())
}

pub async fn get_folder_metadata(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    path: Option<String>,
    folder_id: Option<i64>,
) -> Result<StorageFolderMetadataResult, ApiError> {
    let (current_folder, current_access) = if let Some(folder_id) = folder_id {
        load_accessible_folder_by_id(pool, current_user.user.id, folder_id).await?
    } else {
        let requested_path = normalize_api_path(path.as_deref().unwrap_or("/"))?;
        (
            load_requested_folder(pool, current_user.user.id, &requested_path).await?,
            AccessLevel::Owner,
        )
    };

    let metadata = sqlx::query_as::<_, FolderMetadataRow>(
        r#"
        SELECT
            folder.name,
            folder.path,
            owner_user.username AS owner_username,
            (EXTRACT(EPOCH FROM folder.created_at) * 1000)::BIGINT AS created_at_unix_ms,
            (EXTRACT(EPOCH FROM folder.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
            (
                SELECT COUNT(*)::BIGINT
                FROM folders child
                WHERE child.parent_folder_id = folder.id
                  AND child.is_deleted = false
            ) AS folder_count,
            (
                SELECT COUNT(*)::BIGINT
                FROM files file_row
                WHERE file_row.folder_id = folder.id
                  AND file_row.is_deleted = false
            ) AS file_count
        FROM folders folder
        INNER JOIN users owner_user
            ON owner_user.id = folder.owner_user_id
        WHERE folder.id = $1
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
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
        owner_username: metadata.owner_username,
        current_privilege: current_access.as_str().to_owned(),
        created_at_unix_ms: metadata.created_at_unix_ms,
        modified_at_unix_ms: metadata.modified_at_unix_ms,
        folder_count: metadata.folder_count,
        file_count: metadata.file_count,
        total_item_count: metadata.folder_count + metadata.file_count,
    })
}

pub async fn get_file_metadata(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    file_id: i64,
) -> Result<StorageFileMetadataResult, ApiError> {
    let file = sqlx::query_as::<_, FileMetadataRow>(
        r#"
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_folder_id, owner_user_id
            FROM folders
            WHERE id = (
                SELECT folder_id
                FROM files
                WHERE id = $1
                LIMIT 1
            )
              AND is_deleted = false

            UNION ALL

            SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
            FROM folders parent
            INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
            WHERE parent.is_deleted = false
        )
        SELECT
            file_row.id,
            folder.id AS folder_id,
            folder.path AS folder_path,
            file_row.owner_user_id,
            owner_user.username AS owner_username,
            file_row.name,
            CASE
                WHEN folder.path = '/' THEN '/' || file_row.name
                ELSE folder.path || '/' || file_row.name
            END AS path,
            file_row.size_bytes,
            file_row.mime_type,
            file_row.extension,
            file_row.is_starred,
            CASE
                WHEN file_row.owner_user_id = $2 THEN 3
                WHEN EXISTS (
                    SELECT 1
                    FROM ancestors branch
                    WHERE branch.owner_user_id = $2
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM file_permissions file_perm
                    WHERE file_perm.file_id = file_row.id
                      AND file_perm.user_id = $2
                      AND lower(file_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM file_permissions file_perm
                    WHERE file_perm.file_id = file_row.id
                      AND file_perm.user_id = $2
                      AND lower(file_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                ELSE 0
            END AS access_rank,
            (EXTRACT(EPOCH FROM file_row.created_at) * 1000)::BIGINT AS created_at_unix_ms,
            (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        INNER JOIN users owner_user
            ON owner_user.id = file_row.owner_user_id
        WHERE file_row.id = $1
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(file_id)
    .bind(current_user.user.id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load file metadata"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?;

    let access = access_level_from_rank(file.access_rank).ok_or_else(|| {
        ApiError::BadRequest("You do not have access to the requested file".to_owned())
    })?;

    Ok(StorageFileMetadataResult {
        id: file.id,
        folder_id: file.folder_id,
        folder_path: normalize_db_path(&file.folder_path),
        owner_user_id: file.owner_user_id,
        owner_username: file.owner_username,
        current_privilege: access.as_str().to_owned(),
        name: file.name,
        path: normalize_db_path(&file.path),
        size_bytes: file.size_bytes,
        mime_type: file.mime_type,
        extension: file.extension,
        is_starred: file.is_starred,
        created_at_unix_ms: file.created_at_unix_ms,
        modified_at_unix_ms: file.modified_at_unix_ms,
    })
}

pub async fn resolve_file_download(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    query: DownloadFileQuery,
) -> Result<DownloadFileResult, ApiError> {
    let settings = load_system_storage_settings(pool).await?;

    let file_row = if let Some(file_id) = query.file_id {
        sqlx::query_as::<_, DownloadFileRow>(
            r#"
            WITH RECURSIVE ancestors AS (
                SELECT id, parent_folder_id, owner_user_id
                FROM folders
                WHERE id = (
                    SELECT folder_id
                    FROM files
                    WHERE id = $1
                    LIMIT 1
                )
                  AND is_deleted = false

                UNION ALL

                SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
                FROM folders parent
                INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
                WHERE parent.is_deleted = false
            )
            SELECT
                file_row.storage_path,
                file_row.mime_type,
                file_row.original_file_name,
                file_row.name
            FROM files file_row
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            WHERE file_row.id = $1
              AND file_row.is_deleted = false
              AND folder.is_deleted = false
              AND (
                    file_row.owner_user_id = $2
                    OR EXISTS (
                        SELECT 1
                        FROM ancestors branch
                        WHERE branch.owner_user_id = $2
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM file_permissions file_perm
                        WHERE file_perm.file_id = file_row.id
                          AND file_perm.user_id = $2
                          AND lower(file_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                    )
                    OR EXISTS (
                        SELECT 1
                        FROM folder_permissions folder_perm
                        INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                        WHERE folder_perm.user_id = $2
                          AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                    )
              )
            LIMIT 1
            "#,
        )
        .bind(file_id)
        .bind(current_user.user.id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to resolve file download path"))?
        .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?
    } else {
        let requested_path = normalize_api_path(query.path.as_deref().unwrap_or("/"))?;
        if requested_path == "/" {
            return Err(ApiError::BadRequest(
                "Requested path must point to a file".to_owned(),
            ));
        }

        sqlx::query_as::<_, DownloadFileRow>(
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
        .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?
    };

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

pub async fn build_batch_download_archive(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    items: Vec<BatchDownloadItemInput>,
) -> Result<BatchDownloadResult, ApiError> {
    if items.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one resource must be selected for batch download".to_owned(),
        ));
    }

    if items.len() > 1000 {
        return Err(ApiError::BadRequest(
            "Too many resources selected for one batch download".to_owned(),
        ));
    }

    let settings = load_system_storage_settings(pool).await?;
    let mut archive_sources: Vec<(PathBuf, String)> = Vec::new();
    let mut seen_resource_ids: HashSet<(i64, &'static str)> = HashSet::new();

    for item in items {
        let kind = match item.resource_type {
            StorageEntryKind::Folder => "folder",
            StorageEntryKind::File => "file",
        };

        if !seen_resource_ids.insert((item.resource_id, kind)) {
            continue;
        }

        match item.resource_type {
            StorageEntryKind::Folder => {
                let (folder, _access) =
                    load_accessible_folder_by_id(pool, current_user.user.id, item.resource_id)
                        .await?;

                let folder_label = folder_download_label(&folder.path);
                let folder_files = load_folder_files_for_batch_download(
                    pool,
                    folder.owner_user_id,
                    &folder.path,
                )
                .await?;

                for file in folder_files {
                    let absolute = PathBuf::from(&settings.storage_root_path)
                        .join(file.storage_path.trim_start_matches('/'));

                    if !absolute.is_file() {
                        continue;
                    }

                    let relative_inside_folder =
                        strip_folder_prefix(&file.logical_path, &folder.path);
                    let entry_path = if relative_inside_folder.is_empty() {
                        folder_label.clone()
                    } else {
                        format!("{folder_label}/{relative_inside_folder}")
                    };

                    archive_sources.push((absolute, entry_path));
                }
            }
            StorageEntryKind::File => {
                let file = load_accessible_file_by_id_for_batch_download(
                    pool,
                    current_user.user.id,
                    item.resource_id,
                )
                .await?;

                let absolute =
                    PathBuf::from(&settings.storage_root_path).join(file.storage_path.trim_start_matches('/'));
                if !absolute.is_file() {
                    continue;
                }

                archive_sources.push((absolute, file_download_label(&file.logical_path)));
            }
        }
    }

    if archive_sources.is_empty() {
        return Err(ApiError::BadRequest(
            "No downloadable files were found for the selected resources".to_owned(),
        ));
    }

    let unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0);
    let archive_name = format!("pcloud-batch-{unix_seconds}.zip");
    let archive_root = PathBuf::from(&settings.storage_root_path)
        .join("app-data")
        .join("tmp")
        .join("batch-downloads");

    fs::create_dir_all(&archive_root).map_err(|_| {
        ApiError::internal_with_context("Failed to create temporary archive directory")
    })?;

    let mut random = [0_u8; 8];
    OsRng.fill_bytes(&mut random);
    let random_hex = hex::encode(random);
    let archive_path = archive_root.join(format!(
        "batch-u{}-{unix_seconds}-{random_hex}.zip",
        current_user.user.id
    ));

    let archive_path_for_task = archive_path.clone();

    let build_task = tokio::task::spawn_blocking(move || -> Result<u64, ApiError> {
        let archive_file = std::fs::File::create(&archive_path_for_task).map_err(|_| {
            ApiError::internal_with_context("Failed to create temporary archive file")
        })?;
        let mut zip_writer = ZipWriter::new(archive_file);
        let file_options = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .unix_permissions(0o644);

        let mut used_paths: HashSet<String> = HashSet::new();

        for (absolute_path, raw_zip_path) in archive_sources {
            let normalized_path = normalize_zip_entry_path(&raw_zip_path);
            if normalized_path.is_empty() {
                continue;
            }

            let unique_path = make_unique_zip_entry_path(&normalized_path, &mut used_paths);

            zip_writer
                .start_file(&unique_path, file_options)
                .map_err(|_| ApiError::internal_with_context("Failed to create archive entry"))?;

            let mut source_file = std::fs::File::open(&absolute_path).map_err(|_| {
                ApiError::internal_with_context("Failed to open file for archive creation")
            })?;
            std::io::copy(&mut source_file, &mut zip_writer).map_err(|_| {
                ApiError::internal_with_context("Failed to write file into archive")
            })?;
        }

        let archive_file = zip_writer
            .finish()
            .map_err(|_| ApiError::internal_with_context("Failed to finalize download archive"))?;
        let archive_size_bytes = archive_file
            .metadata()
            .map_err(|_| {
                ApiError::internal_with_context("Failed to read temporary archive file metadata")
            })?
            .len();

        Ok(archive_size_bytes)
    })
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to build batch download archive"))?;

    let archive_size_bytes = match build_task {
        Ok(size) => size,
        Err(error) => {
            let _ = fs::remove_file(&archive_path);
            return Err(error);
        }
    };

    Ok(BatchDownloadResult {
        archive_name,
        archive_path,
        archive_size_bytes,
    })
}

pub async fn create_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: CreateFolderInput,
) -> Result<StorageEntryDto, ApiError> {
    let folder_name = normalize_item_name(&input.name, "Folder name")?;
    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start folder creation transaction")
    })?;

    let (parent_folder, access) = if let Some(parent_folder_id) = input.parent_folder_id {
        load_accessible_folder_by_id_tx(&mut tx, current_user.user.id, parent_folder_id).await?
    } else {
        let parent_path = normalize_api_path(input.parent_path.as_deref().unwrap_or("/"))?;
        (
            load_requested_folder_tx(&mut tx, current_user.user.id, &parent_path).await?,
            AccessLevel::Owner,
        )
    };

    if !access.is_editor() {
        return Err(ApiError::BadRequest(
            "You need editor permission to create a folder here".to_owned(),
        ));
    }

    ensure_name_not_taken_tx(&mut tx, parent_folder.id, &folder_name).await?;

    let child_path = join_child_path(&normalize_db_path(&parent_folder.path), &folder_name);
    // Ownership follows the parent folder owner so shared editors create resources
    // under the parent owner's namespace and quota.
    let owner_user_id = parent_folder.owner_user_id;

    let inserted = sqlx::query_as::<_, StorageEntryRow>(
        r#"
        INSERT INTO folders (owner_user_id, created_by_user_id, parent_folder_id, name, path, is_deleted)
        VALUES ($1, $2, $3, $4, $5, false)
        RETURNING
            id,
            name,
            path,
            'folder'::TEXT AS entry_type,
            owner_user_id,
            (SELECT username FROM users WHERE users.id = owner_user_id) AS owner_username,
            created_by_user_id,
            COALESCE((SELECT username FROM users WHERE users.id = created_by_user_id), 'Deleted user') AS created_by_username,
            is_starred,
            NULL::BIGINT AS size_bytes,
            (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        "#,
    )
    .bind(owner_user_id)
    .bind(current_user.user.id)
    .bind(parent_folder.id)
    .bind(&folder_name)
    .bind(&child_path)
    .fetch_one(&mut *tx)
    .await
    .map_err(map_storage_write_error)?;

    let user_root = resolve_user_storage_root(&settings.storage_root_path, owner_user_id);
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

    let (target_folder, access) = if let Some(folder_id) = input.folder_id {
        match load_accessible_folder_by_id_tx(&mut tx, current_user.user.id, folder_id).await {
            Ok(value) => value,
            Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
        }
    } else {
        let folder_path = match normalize_api_path(input.folder_path.as_deref().unwrap_or("/")) {
            Ok(value) => value,
            Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
        };

        match load_requested_folder_tx(&mut tx, current_user.user.id, &folder_path).await {
            Ok(value) => (value, AccessLevel::Owner),
            Err(error) => return Err(cleanup_temp_file_and_return(&temp_file_path, error)),
        }
    };

    if !access.is_editor() {
        return Err(cleanup_temp_file_and_return(
            &temp_file_path,
            ApiError::BadRequest("You need editor permission to upload files here".to_owned()),
        ));
    }

    if let Err(error) = ensure_name_not_taken_tx(&mut tx, target_folder.id, &file_name).await {
        return Err(cleanup_temp_file_and_return(&temp_file_path, error));
    }

    // Ownership follows the parent folder owner so shared editors upload files
    // under the parent owner's namespace and quota.
    let owner_user_id = target_folder.owner_user_id;

    let user_storage = match load_user_storage_tx(&mut tx, owner_user_id).await {
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

    let user_root = resolve_user_storage_root(&settings.storage_root_path, owner_user_id);
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
        .join(owner_user_id.to_string())
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
            created_by_user_id,
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
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, false)
        RETURNING
            id,
            name,
            CASE
                WHEN $11::TEXT = '/' THEN '/' || name
                ELSE $11::TEXT || '/' || name
            END AS path,
            'file'::TEXT AS entry_type,
            owner_user_id,
            (SELECT username FROM users WHERE users.id = owner_user_id) AS owner_username,
            created_by_user_id,
            COALESCE((SELECT username FROM users WHERE users.id = created_by_user_id), 'Deleted user') AS created_by_username,
            is_starred,
            size_bytes,
            (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        "#,
    )
    .bind(owner_user_id)
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
    .bind(owner_user_id)
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

pub async fn move_storage_entries(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: MoveStorageInput,
) -> Result<MoveStorageResult, ApiError> {
    if input.destination_folder_id <= 0 {
        return Err(ApiError::BadRequest(
            "destinationFolderId must be a positive integer".to_owned(),
        ));
    }

    if input.items.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one item must be selected for move".to_owned(),
        ));
    }

    if input.items.len() > 500 {
        return Err(ApiError::BadRequest(
            "Move request is too large (maximum is 500 items)".to_owned(),
        ));
    }

    let settings = load_system_storage_settings(pool).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to start move transaction"))?;

    let destination_folder =
        load_owned_folder_for_move_by_id_tx(&mut tx, current_user.user.id, input.destination_folder_id)
            .await?;

    let mut dedupe = HashSet::<String>::new();
    let mut requested_items = Vec::<MoveStorageItemInput>::new();
    for item in input.items {
        if item.resource_id <= 0 {
            return Err(ApiError::BadRequest(
                "resourceId must be a positive integer".to_owned(),
            ));
        }

        let item_type = match item.resource_type {
            StorageEntryKind::Folder => "folder",
            StorageEntryKind::File => "file",
        };

        let key = format!("{item_type}:{}", item.resource_id);
        if dedupe.insert(key) {
            requested_items.push(item);
        }
    }

    if requested_items.is_empty() {
        return Err(ApiError::BadRequest(
            "At least one unique item must be selected for move".to_owned(),
        ));
    }

    let mut folder_sources = Vec::<MoveFolderRow>::new();
    let mut file_sources = Vec::<MoveFileRow>::new();

    for item in requested_items {
        match item.resource_type {
            StorageEntryKind::Folder => {
                let folder =
                    load_owned_folder_for_move_by_id_tx(&mut tx, current_user.user.id, item.resource_id)
                        .await?;
                folder_sources.push(folder);
            }
            StorageEntryKind::File => {
                let file =
                    load_owned_file_for_move_by_id_tx(&mut tx, current_user.user.id, item.resource_id)
                        .await?;
                file_sources.push(file);
            }
        }
    }

    folder_sources.sort_by(|left, right| {
        left.path
            .len()
            .cmp(&right.path.len())
            .then(left.path.cmp(&right.path))
            .then(left.id.cmp(&right.id))
    });

    let mut effective_folders = Vec::<MoveFolderRow>::new();
    for folder in folder_sources {
        let covered_by_parent_selection = effective_folders
            .iter()
            .any(|ancestor| is_descendant_path(&folder.path, &ancestor.path));
        if covered_by_parent_selection {
            continue;
        }
        effective_folders.push(folder);
    }

    let mut effective_files = Vec::<MoveFileRow>::new();
    for file in file_sources {
        let file_path = join_child_path(&normalize_db_path(&file.folder_path), &file.name);
        let covered_by_folder_selection = effective_folders
            .iter()
            .any(|folder| is_descendant_path(&file_path, &folder.path));
        if covered_by_folder_selection {
            continue;
        }
        effective_files.push(file);
    }

    let mut filesystem_moves = Vec::<(PathBuf, PathBuf)>::new();
    let user_root = resolve_user_storage_root(&settings.storage_root_path, current_user.user.id);
    let mut moved_count = 0_i64;

    for folder in &effective_folders {
        if folder.parent_folder_id.is_none() {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::BadRequest(
                "Root folder cannot be moved".to_owned(),
            ));
        }

        if folder.id == destination_folder.id {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::BadRequest(
                "Folder cannot be moved into itself".to_owned(),
            ));
        }

        if is_descendant_path(&destination_folder.path, &folder.path) {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::BadRequest(
                "Folder cannot be moved into its child folder".to_owned(),
            ));
        }

        if folder.parent_folder_id == Some(destination_folder.id) {
            continue;
        }

        ensure_name_not_taken_tx(&mut tx, destination_folder.id, &folder.name).await?;

        let new_folder_path = join_child_path(&destination_folder.path, &folder.name);
        let old_folder_prefix = format!("{}/%", folder.path.trim_end_matches('/'));
        let old_storage_prefix = logical_path_to_storage_prefix(folder.owner_user_id, &folder.path);
        let new_storage_prefix = logical_path_to_storage_prefix(folder.owner_user_id, &new_folder_path);
        let old_storage_prefix_like = format!("{}/%", old_storage_prefix.trim_end_matches('/'));

        let old_folder_abs = user_root.join(logical_path_to_relative_path(&folder.path));
        let new_folder_abs = user_root.join(logical_path_to_relative_path(&new_folder_path));

        if new_folder_abs.exists() {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::Conflict(
                "An item with the same name already exists in this folder".to_owned(),
            ));
        }

        if fs::rename(&old_folder_abs, &new_folder_abs).is_err() {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::internal_with_context(
                "Failed to move folder on the storage filesystem",
            ));
        }
        filesystem_moves.push((old_folder_abs.clone(), new_folder_abs.clone()));

        if let Err(error) = sqlx::query(
            r#"
            UPDATE folders
            SET parent_folder_id = $3,
                updated_at = NOW()
            WHERE id = $1
              AND owner_user_id = $2
              AND is_deleted = false
            "#,
        )
        .bind(folder.id)
        .bind(folder.owner_user_id)
        .bind(destination_folder.id)
        .execute(&mut *tx)
        .await
        {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(map_storage_write_error(error));
        }

        if let Err(error) = sqlx::query(
            r#"
            UPDATE folders
            SET path = CASE
                WHEN path = $2 THEN $3
                ELSE $3 || SUBSTRING(path FROM CHAR_LENGTH($2) + 1)
            END,
                updated_at = NOW()
            WHERE owner_user_id = $1
              AND is_deleted = false
              AND (
                  path = $2
                  OR path LIKE $4
              )
            "#,
        )
        .bind(folder.owner_user_id)
        .bind(&folder.path)
        .bind(&new_folder_path)
        .bind(&old_folder_prefix)
        .execute(&mut *tx)
        .await
        {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(map_storage_write_error(error));
        }

        if let Err(error) = sqlx::query(
            r#"
            UPDATE files
            SET storage_path = CASE
                WHEN storage_path = $2 THEN $3
                ELSE $3 || SUBSTRING(storage_path FROM CHAR_LENGTH($2) + 1)
            END,
                updated_at = NOW()
            WHERE owner_user_id = $1
              AND is_deleted = false
              AND (
                  storage_path = $2
                  OR storage_path LIKE $4
              )
            "#,
        )
        .bind(folder.owner_user_id)
        .bind(&old_storage_prefix)
        .bind(&new_storage_prefix)
        .bind(&old_storage_prefix_like)
        .execute(&mut *tx)
        .await
        {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(map_storage_write_error(error));
        }

        moved_count += 1;
    }

    for file in &effective_files {
        if file.folder_id == destination_folder.id {
            continue;
        }

        ensure_name_not_taken_tx(&mut tx, destination_folder.id, &file.name).await?;

        let old_storage_rel = PathBuf::from(file.storage_path.trim_start_matches('/'));
        let new_storage_rel = Path::new("users")
            .join(file.owner_user_id.to_string())
            .join(logical_path_to_relative_path(&destination_folder.path))
            .join(&file.name);

        let new_storage_path = format!("/{}", new_storage_rel.to_string_lossy().replace('\\', "/"));
        let old_absolute_path = PathBuf::from(&settings.storage_root_path).join(&old_storage_rel);
        let new_absolute_path = PathBuf::from(&settings.storage_root_path).join(&new_storage_rel);

        if new_absolute_path.exists() {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::Conflict(
                "An item with the same name already exists in this folder".to_owned(),
            ));
        }

        if fs::rename(&old_absolute_path, &new_absolute_path).is_err() {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(ApiError::internal_with_context(
                "Failed to move file on the storage filesystem",
            ));
        }
        filesystem_moves.push((old_absolute_path.clone(), new_absolute_path.clone()));

        if let Err(error) = sqlx::query(
            r#"
            UPDATE files
            SET folder_id = $3,
                storage_path = $4,
                updated_at = NOW()
            WHERE id = $1
              AND owner_user_id = $2
              AND is_deleted = false
            "#,
        )
        .bind(file.id)
        .bind(file.owner_user_id)
        .bind(destination_folder.id)
        .bind(&new_storage_path)
        .execute(&mut *tx)
        .await
        {
            rollback_filesystem_moves(&filesystem_moves);
            return Err(map_storage_write_error(error));
        }

        moved_count += 1;
    }

    if moved_count == 0 {
        rollback_filesystem_moves(&filesystem_moves);
        return Err(ApiError::BadRequest(
            "No items were moved. Choose a different destination".to_owned(),
        ));
    }

    if tx.commit().await.is_err() {
        rollback_filesystem_moves(&filesystem_moves);
        return Err(ApiError::internal_with_context(
            "Failed to commit move transaction",
        ));
    }

    Ok(MoveStorageResult {
        moved_count,
        destination_folder_id: destination_folder.id,
        destination_path: normalize_db_path(&destination_folder.path),
    })
}

pub async fn rename_file(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: RenameStorageInput,
) -> Result<StorageEntryDto, ApiError> {
    let new_name = normalize_item_name(&input.new_name, "File name")?;
    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool
        .begin()
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to start file rename transaction"))?;

    let (target_file, access) = if let Some(resource_id) = input.resource_id {
        load_accessible_file_for_rename_by_id_tx(&mut tx, current_user.user.id, resource_id).await?
    } else {
        let requested_path = normalize_api_path(input.path.as_deref().unwrap_or("/"))?;
        if requested_path == "/" {
            return Err(ApiError::BadRequest(
                "Requested path must point to a file".to_owned(),
            ));
        }

        (
            load_file_for_rename_tx(&mut tx, current_user.user.id, &requested_path).await?,
            AccessLevel::Owner,
        )
    };

    if !access.is_editor() {
        return Err(ApiError::BadRequest(
            "You need editor permission to rename files here".to_owned(),
        ));
    }

    if target_file.name == new_name {
        let unchanged_path = join_child_path(
            &normalize_db_path(&target_file.folder_path),
            &target_file.name,
        );

        return Ok(StorageEntryDto {
            id: target_file.id,
            name: target_file.name,
            path: unchanged_path,
            entry_type: "file".to_owned(),
            owner_user_id: target_file.owner_user_id,
            owner_username: lookup_username_by_id_tx(&mut tx, target_file.owner_user_id).await?,
            created_by_user_id: target_file.created_by_user_id,
            created_by_username: lookup_optional_username_by_id_tx(
                &mut tx,
                target_file.created_by_user_id,
            )
            .await?,
            is_starred: target_file.is_starred,
            size_bytes: None,
            modified_at_unix_ms: None,
        });
    }

    ensure_name_not_taken_tx(&mut tx, target_file.folder_id, &new_name).await?;

    let old_storage_rel = PathBuf::from(target_file.storage_path.trim_start_matches('/'));
    let new_storage_rel = old_storage_rel
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            ApiError::internal_with_context("Invalid stored file path for rename operation")
        })?
        .join(&new_name);

    let new_storage_path = format!("/{}", new_storage_rel.to_string_lossy().replace('\\', "/"));
    let old_absolute_path = PathBuf::from(&settings.storage_root_path).join(&old_storage_rel);
    let new_absolute_path = PathBuf::from(&settings.storage_root_path).join(&new_storage_rel);

    if new_absolute_path.exists() {
        return Err(ApiError::Conflict(
            "An item with the same name already exists in this folder".to_owned(),
        ));
    }

    if fs::rename(&old_absolute_path, &new_absolute_path).is_err() {
        return Err(ApiError::internal_with_context(
            "Failed to rename file on the storage filesystem",
        ));
    }

    let updated = match sqlx::query_as::<_, StorageEntryRow>(
        r#"
        UPDATE files
        SET name = $3,
            original_file_name = $3,
            storage_path = $4,
            updated_at = NOW()
        WHERE id = $1
          AND owner_user_id = $2
          AND is_deleted = false
        RETURNING
            id,
            name,
            CASE
                WHEN $5::TEXT = '/' THEN '/' || name
                ELSE $5::TEXT || '/' || name
                END AS path,
                'file'::TEXT AS entry_type,
                owner_user_id,
                (SELECT username FROM users WHERE users.id = owner_user_id) AS owner_username,
                created_by_user_id,
                COALESCE((SELECT username FROM users WHERE users.id = created_by_user_id), 'Deleted user') AS created_by_username,
                is_starred,
                size_bytes,
                (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        "#,
    )
    .bind(target_file.id)
    .bind(target_file.owner_user_id)
    .bind(&new_name)
    .bind(&new_storage_path)
    .bind(normalize_db_path(&target_file.folder_path))
    .fetch_one(&mut *tx)
    .await
    {
        Ok(entry) => entry,
        Err(error) => {
            let _ = fs::rename(&new_absolute_path, &old_absolute_path);
            return Err(map_storage_write_error(error));
        }
    };

    if tx.commit().await.is_err() {
        let _ = fs::rename(&new_absolute_path, &old_absolute_path);
        return Err(ApiError::internal_with_context(
            "Failed to commit file rename transaction",
        ));
    }

    Ok(storage_entry_row_to_dto(updated))
}

pub async fn rename_folder(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: RenameStorageInput,
) -> Result<StorageEntryDto, ApiError> {
    let new_name = normalize_item_name(&input.new_name, "Folder name")?;
    let settings = load_system_storage_settings(pool).await?;

    let mut tx = pool.begin().await.map_err(|_| {
        ApiError::internal_with_context("Failed to start folder rename transaction")
    })?;

    let (target_folder, access) = if let Some(resource_id) = input.resource_id {
        let (folder_row, folder_access) =
            load_accessible_folder_by_id_tx(&mut tx, current_user.user.id, resource_id).await?;
        let rename_row = load_folder_for_rename_by_id_tx(&mut tx, folder_row.id).await?;
        (rename_row, folder_access)
    } else {
        let requested_path = normalize_api_path(input.path.as_deref().unwrap_or("/"))?;
        if requested_path == "/" {
            return Err(ApiError::BadRequest(
                "Root folder cannot be renamed".to_owned(),
            ));
        }

        (
            load_folder_for_rename_tx(&mut tx, current_user.user.id, &requested_path).await?,
            AccessLevel::Owner,
        )
    };

    if !access.is_editor() {
        return Err(ApiError::BadRequest(
            "You need editor permission to rename folders here".to_owned(),
        ));
    }

    if target_folder.name == new_name {
        return Ok(StorageEntryDto {
            id: target_folder.id,
            name: target_folder.name,
            path: normalize_db_path(&target_folder.path),
            entry_type: "folder".to_owned(),
            owner_user_id: target_folder.owner_user_id,
            owner_username: lookup_username_by_id_tx(&mut tx, target_folder.owner_user_id).await?,
            created_by_user_id: target_folder.created_by_user_id,
            created_by_username: lookup_optional_username_by_id_tx(
                &mut tx,
                target_folder.created_by_user_id,
            )
            .await?,
            is_starred: target_folder.is_starred,
            size_bytes: None,
            modified_at_unix_ms: None,
        });
    }

    let parent_folder_id = target_folder
        .parent_folder_id
        .ok_or_else(|| ApiError::BadRequest("Root folder cannot be renamed".to_owned()))?;

    ensure_name_not_taken_tx(&mut tx, parent_folder_id, &new_name).await?;

    let parent_path = parent_api_path(&target_folder.path)
        .ok_or_else(|| ApiError::BadRequest("Root folder cannot be renamed".to_owned()))?;
    let new_folder_path = join_child_path(&parent_path, &new_name);
    let old_folder_prefix = format!("{}/%", target_folder.path.trim_end_matches('/'));

    let old_storage_prefix =
        logical_path_to_storage_prefix(target_folder.owner_user_id, &target_folder.path);
    let new_storage_prefix =
        logical_path_to_storage_prefix(target_folder.owner_user_id, &new_folder_path);
    let old_storage_prefix_like = format!("{}/%", old_storage_prefix.trim_end_matches('/'));

    let user_root =
        resolve_user_storage_root(&settings.storage_root_path, target_folder.owner_user_id);
    let old_folder_abs = user_root.join(logical_path_to_relative_path(&target_folder.path));
    let new_folder_abs = user_root.join(logical_path_to_relative_path(&new_folder_path));

    if new_folder_abs.exists() {
        return Err(ApiError::Conflict(
            "An item with the same name already exists in this folder".to_owned(),
        ));
    }

    if fs::rename(&old_folder_abs, &new_folder_abs).is_err() {
        return Err(ApiError::internal_with_context(
            "Failed to rename folder on the storage filesystem",
        ));
    }

    if let Err(error) = sqlx::query(
        r#"
        UPDATE folders
        SET name = $3,
            updated_at = NOW()
        WHERE id = $1
          AND owner_user_id = $2
        "#,
    )
    .bind(target_folder.id)
    .bind(target_folder.owner_user_id)
    .bind(&new_name)
    .execute(&mut *tx)
    .await
    {
        let _ = fs::rename(&new_folder_abs, &old_folder_abs);
        return Err(map_storage_write_error(error));
    }

    if let Err(error) = sqlx::query(
        r#"
        UPDATE folders
        SET path = CASE
            WHEN path = $2 THEN $3
            ELSE $3 || SUBSTRING(path FROM CHAR_LENGTH($2) + 1)
        END,
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND (
              path = $2
              OR path LIKE $4
          )
        "#,
    )
    .bind(target_folder.owner_user_id)
    .bind(&target_folder.path)
    .bind(&new_folder_path)
    .bind(&old_folder_prefix)
    .execute(&mut *tx)
    .await
    {
        let _ = fs::rename(&new_folder_abs, &old_folder_abs);
        return Err(map_storage_write_error(error));
    }

    if let Err(error) = sqlx::query(
        r#"
        UPDATE files
        SET storage_path = CASE
            WHEN storage_path = $2 THEN $3
            ELSE $3 || SUBSTRING(storage_path FROM CHAR_LENGTH($2) + 1)
        END,
            updated_at = NOW()
        WHERE owner_user_id = $1
          AND (
              storage_path = $2
              OR storage_path LIKE $4
          )
        "#,
    )
    .bind(target_folder.owner_user_id)
    .bind(&old_storage_prefix)
    .bind(&new_storage_prefix)
    .bind(&old_storage_prefix_like)
    .execute(&mut *tx)
    .await
    {
        let _ = fs::rename(&new_folder_abs, &old_folder_abs);
        return Err(map_storage_write_error(error));
    }

    let renamed_folder = match sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            id,
            name,
            path,
            'folder'::TEXT AS entry_type,
            owner_user_id,
            (SELECT username FROM users WHERE users.id = owner_user_id) AS owner_username,
            created_by_user_id,
            COALESCE((SELECT username FROM users WHERE users.id = created_by_user_id), 'Deleted user') AS created_by_username,
            is_starred,
            NULL::BIGINT AS size_bytes,
            (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
        FROM folders
        WHERE id = $1
          AND owner_user_id = $2
        LIMIT 1
        "#,
    )
    .bind(target_folder.id)
    .bind(target_folder.owner_user_id)
    .fetch_optional(&mut *tx)
    .await
    {
        Ok(Some(entry)) => entry,
        Ok(None) => {
            let _ = fs::rename(&new_folder_abs, &old_folder_abs);
            return Err(ApiError::BadRequest(
                "Requested folder does not exist".to_owned(),
            ));
        }
        Err(error) => {
            let _ = fs::rename(&new_folder_abs, &old_folder_abs);
            return Err(map_storage_write_error(error));
        }
    };

    if tx.commit().await.is_err() {
        let _ = fs::rename(&new_folder_abs, &old_folder_abs);
        return Err(ApiError::internal_with_context(
            "Failed to commit folder rename transaction",
        ));
    }

    Ok(storage_entry_row_to_dto(renamed_folder))
}

pub async fn set_starred(
    pool: &PgPool,
    current_user: &AuthenticatedUser,
    input: SetStarredInput,
) -> Result<StorageEntryDto, ApiError> {
    let requested_path = normalize_api_path(input.path.as_deref().unwrap_or("/"))?;

    match input.entry_type {
        StorageEntryKind::Folder => {
            if requested_path == "/" {
                return Err(ApiError::BadRequest(
                    "Root folder cannot be starred".to_owned(),
                ));
            }

            let updated = sqlx::query_as::<_, StorageEntryRow>(
                r#"
                UPDATE folders
                SET is_starred = $3,
                    updated_at = NOW()
                WHERE owner_user_id = $1
                  AND path = $2
                  AND is_deleted = false
                RETURNING
                    id,
                    name,
                    path,
                    'folder'::TEXT AS entry_type,
                    owner_user_id,
                    (SELECT username FROM users WHERE users.id = owner_user_id) AS owner_username,
                    created_by_user_id,
                    COALESCE((SELECT username FROM users WHERE users.id = created_by_user_id), 'Deleted user') AS created_by_username,
                    is_starred,
                    NULL::BIGINT AS size_bytes,
                    (EXTRACT(EPOCH FROM updated_at) * 1000)::BIGINT AS modified_at_unix_ms
                "#,
            )
            .bind(current_user.user.id)
            .bind(&requested_path)
            .bind(input.starred)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal_with_context("Failed to update folder star status"))?
            .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist".to_owned()))?;

            Ok(storage_entry_row_to_dto(updated))
        }
        StorageEntryKind::File => {
            if requested_path == "/" {
                return Err(ApiError::BadRequest(
                    "Requested path must point to a file".to_owned(),
                ));
            }

            let updated = sqlx::query_as::<_, StorageEntryRow>(
                r#"
                UPDATE files
                SET is_starred = $3,
                    updated_at = NOW()
                FROM folders folder
                WHERE files.owner_user_id = $1
                  AND folder.owner_user_id = $1
                  AND files.folder_id = folder.id
                  AND files.is_deleted = false
                  AND folder.is_deleted = false
                  AND (
                      CASE
                          WHEN folder.path = '/' THEN '/' || files.name
                          ELSE folder.path || '/' || files.name
                      END
                  ) = $2
                RETURNING
                    files.id,
                    files.name,
                    CASE
                        WHEN folder.path = '/' THEN '/' || files.name
                        ELSE folder.path || '/' || files.name
                    END AS path,
                    'file'::TEXT AS entry_type,
                    files.owner_user_id,
                    (SELECT username FROM users WHERE users.id = files.owner_user_id) AS owner_username,
                    files.created_by_user_id,
                    COALESCE((SELECT username FROM users WHERE users.id = files.created_by_user_id), 'Deleted user') AS created_by_username,
                    files.is_starred,
                    files.size_bytes,
                    (EXTRACT(EPOCH FROM files.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
                "#,
            )
            .bind(current_user.user.id)
            .bind(&requested_path)
            .bind(input.starred)
            .fetch_optional(pool)
            .await
            .map_err(|_| ApiError::internal_with_context("Failed to update file star status"))?
            .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?;

            Ok(storage_entry_row_to_dto(updated))
        }
    }
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
            SELECT id, path, parent_folder_id, owner_user_id
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
        SELECT id, path, parent_folder_id, owner_user_id
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

async fn load_accessible_folder_by_id(
    pool: &PgPool,
    user_id: i64,
    folder_id: i64,
) -> Result<(FolderRow, AccessLevel), ApiError> {
    let row = sqlx::query_as::<_, FolderAccessRow>(
        r#"
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_folder_id, owner_user_id
            FROM folders
            WHERE id = $1
              AND is_deleted = false

            UNION ALL

            SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
            FROM folders parent
            INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
            WHERE parent.is_deleted = false
        )
        SELECT
            folder.id,
            folder.path,
            folder.parent_folder_id,
            folder.owner_user_id,
            CASE
                WHEN folder.owner_user_id = $2 THEN 3
                WHEN EXISTS (
                    SELECT 1
                    FROM ancestors branch
                    WHERE branch.owner_user_id = $2
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                ELSE 0
            END AS access_rank
        FROM folders folder
        WHERE folder.id = $1
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(folder_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve storage folder access"))?
    .ok_or_else(|| ApiError::BadRequest("Requested storage folder does not exist".to_owned()))?;

    let access = access_level_from_rank(row.access_rank)
        .ok_or_else(|| ApiError::BadRequest("You do not have access to this folder".to_owned()))?;

    Ok((
        FolderRow {
            id: row.id,
            path: row.path,
            parent_folder_id: row.parent_folder_id,
            owner_user_id: row.owner_user_id,
        },
        access,
    ))
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
            SELECT id, path, parent_folder_id, owner_user_id
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
        SELECT id, path, parent_folder_id, owner_user_id
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

async fn load_accessible_folder_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    folder_id: i64,
) -> Result<(FolderRow, AccessLevel), ApiError> {
    let row = sqlx::query_as::<_, FolderAccessRow>(
        r#"
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_folder_id, owner_user_id
            FROM folders
            WHERE id = $1
              AND is_deleted = false

            UNION ALL

            SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
            FROM folders parent
            INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
            WHERE parent.is_deleted = false
        )
        SELECT
            folder.id,
            folder.path,
            folder.parent_folder_id,
            folder.owner_user_id,
            CASE
                WHEN folder.owner_user_id = $2 THEN 3
                WHEN EXISTS (
                    SELECT 1
                    FROM ancestors branch
                    WHERE branch.owner_user_id = $2
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                ELSE 0
            END AS access_rank
        FROM folders folder
        WHERE folder.id = $1
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(folder_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve storage folder access"))?
    .ok_or_else(|| ApiError::BadRequest("Requested storage folder does not exist".to_owned()))?;

    let access = access_level_from_rank(row.access_rank)
        .ok_or_else(|| ApiError::BadRequest("You do not have access to this folder".to_owned()))?;

    Ok((
        FolderRow {
            id: row.id,
            path: row.path,
            parent_folder_id: row.parent_folder_id,
            owner_user_id: row.owner_user_id,
        },
        access,
    ))
}

async fn load_owned_folder_for_move_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    folder_id: i64,
) -> Result<MoveFolderRow, ApiError> {
    sqlx::query_as::<_, MoveFolderRow>(
        r#"
        SELECT
            id,
            owner_user_id,
            name,
            path,
            parent_folder_id
        FROM folders
        WHERE id = $1
          AND owner_user_id = $2
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(folder_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve folder move target"))?
    .ok_or_else(|| {
        ApiError::BadRequest(
            "Requested folder does not exist or does not belong to the current user".to_owned(),
        )
    })
}

async fn load_owned_file_for_move_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    file_id: i64,
) -> Result<MoveFileRow, ApiError> {
    sqlx::query_as::<_, MoveFileRow>(
        r#"
        SELECT
            file_row.id,
            file_row.owner_user_id,
            file_row.name,
            file_row.storage_path,
            file_row.folder_id,
            folder.path AS folder_path
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.id = $1
          AND file_row.owner_user_id = $2
          AND folder.owner_user_id = $2
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(file_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve file move target"))?
    .ok_or_else(|| {
        ApiError::BadRequest(
            "Requested file does not exist or does not belong to the current user".to_owned(),
        )
    })
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

async fn load_file_for_rename_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<RenameFileRow, ApiError> {
    sqlx::query_as::<_, RenameFileRow>(
        r#"
        SELECT
            file_row.id,
            file_row.owner_user_id,
            file_row.created_by_user_id,
            file_row.name,
            file_row.is_starred,
            file_row.storage_path,
            file_row.folder_id,
            folder.path AS folder_path
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
    .map_err(|_| ApiError::internal_with_context("Failed to resolve file rename path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))
}

async fn load_folder_for_rename_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    requested_path: &str,
) -> Result<RenameFolderRow, ApiError> {
    sqlx::query_as::<_, RenameFolderRow>(
        r#"
        SELECT
            id,
            owner_user_id,
            created_by_user_id,
            name,
            is_starred,
            path,
            parent_folder_id
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
    .map_err(|_| ApiError::internal_with_context("Failed to resolve folder rename path"))?
    .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist".to_owned()))
}

async fn load_folder_for_rename_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    folder_id: i64,
) -> Result<RenameFolderRow, ApiError> {
    sqlx::query_as::<_, RenameFolderRow>(
        r#"
        SELECT
            id,
            owner_user_id,
            created_by_user_id,
            name,
            is_starred,
            path,
            parent_folder_id
        FROM folders
        WHERE id = $1
          AND is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(folder_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve folder rename target"))?
    .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist".to_owned()))
}

async fn load_accessible_file_for_rename_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
    file_id: i64,
) -> Result<(RenameFileRow, AccessLevel), ApiError> {
    let row = sqlx::query_as::<_, RenameFileAccessRow>(
        r#"
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_folder_id, owner_user_id
            FROM folders
            WHERE id = (
                SELECT folder_id
                FROM files
                WHERE id = $1
                LIMIT 1
            )
              AND is_deleted = false

            UNION ALL

            SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
            FROM folders parent
            INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
            WHERE parent.is_deleted = false
        )
        SELECT
            file_row.id,
            file_row.owner_user_id,
            file_row.created_by_user_id,
            file_row.name,
            file_row.is_starred,
            file_row.storage_path,
            file_row.folder_id,
            folder.path AS folder_path,
            CASE
                WHEN file_row.owner_user_id = $2 THEN 3
                WHEN EXISTS (
                    SELECT 1
                    FROM ancestors branch
                    WHERE branch.owner_user_id = $2
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM file_permissions file_perm
                    WHERE file_perm.file_id = file_row.id
                      AND file_perm.user_id = $2
                      AND lower(file_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                ) THEN 2
                WHEN EXISTS (
                    SELECT 1
                    FROM file_permissions file_perm
                    WHERE file_perm.file_id = file_row.id
                      AND file_perm.user_id = $2
                      AND lower(file_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                WHEN EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                ) THEN 1
                ELSE 0
            END AS access_rank
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.id = $1
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
        LIMIT 1
        "#,
    )
    .bind(file_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve file rename target"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))?;

    let access = access_level_from_rank(row.access_rank)
        .ok_or_else(|| ApiError::BadRequest("You do not have access to this file".to_owned()))?;

    Ok((
        RenameFileRow {
            id: row.id,
            owner_user_id: row.owner_user_id,
            created_by_user_id: row.created_by_user_id,
            name: row.name,
            is_starred: row.is_starred,
            storage_path: row.storage_path,
            folder_id: row.folder_id,
            folder_path: row.folder_path,
        },
        access,
    ))
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
    parent_folder_id: i64,
    name: &str,
) -> Result<(), ApiError> {
    let name_exists = sqlx::query_scalar::<_, bool>(
        r#"
        SELECT EXISTS (
            SELECT 1
            FROM folders
            WHERE parent_folder_id = $1
              AND name = $2
              AND is_deleted = false
        )
        OR EXISTS (
            SELECT 1
            FROM files
            WHERE folder_id = $1
              AND name = $2
              AND is_deleted = false
        )
        "#,
    )
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
    parent_folder_id: i64,
    parent_folder_path: &str,
    search: Option<&str>,
    page_limit: i64,
    page_offset: i64,
) -> Result<(Vec<StorageEntryRow>, Option<String>, bool), ApiError> {
    let mut rows = sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            entries.id,
            entries.name,
            entries.path,
            entries.entry_type,
            entries.owner_user_id,
            entries.owner_username,
            entries.created_by_user_id,
            entries.created_by_username,
            entries.is_starred,
            entries.size_bytes,
            entries.modified_at_unix_ms
        FROM (
            SELECT
                f.id,
                f.name,
                f.path,
                'folder'::TEXT AS entry_type,
                f.owner_user_id,
                owner_user.username AS owner_username,
                f.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                f.is_starred,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM f.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM folders f
            INNER JOIN users owner_user
                ON owner_user.id = f.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = f.created_by_user_id
            WHERE f.parent_folder_id = $1
              AND f.is_deleted = false
              AND ($2::TEXT IS NULL OR f.name ILIKE '%' || $2 || '%')

            UNION ALL

            SELECT
                file_row.id,
                file_row.name,
                CASE
                    WHEN $3::TEXT = '/' THEN '/' || file_row.name
                    ELSE $3::TEXT || '/' || file_row.name
                END AS path,
                'file'::TEXT AS entry_type,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                file_row.is_starred,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM files file_row
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
            WHERE file_row.folder_id = $1
              AND file_row.is_deleted = false
              AND ($2::TEXT IS NULL OR file_row.name ILIKE '%' || $2 || '%')
        ) entries
        ORDER BY
            CASE WHEN entries.entry_type = 'folder' THEN 0 ELSE 1 END,
            LOWER(entries.name),
            entries.name,
            entries.id
        LIMIT $4
        OFFSET $5
        "#,
    )
    .bind(parent_folder_id)
    .bind(search)
    .bind(parent_folder_path)
    .bind(page_limit + 1)
    .bind(page_offset)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load storage listing"))?;

    let has_more = rows.len() as i64 > page_limit;
    if has_more {
        rows.truncate(page_limit as usize);
    }

    let next_cursor = if has_more {
        Some((page_offset + page_limit).to_string())
    } else {
        None
    };

    Ok((rows, next_cursor, has_more))
}

async fn load_search_entries(
    pool: &PgPool,
    user_id: i64,
    search_pattern: &str,
    page_limit: i64,
    page_offset: i64,
) -> Result<(Vec<SearchResourceRow>, Option<String>, bool), ApiError> {
    let mut rows = sqlx::query_as::<_, SearchResourceRow>(
        r#"
        WITH candidate_entries AS (
            SELECT
                'folder'::TEXT AS resource_type,
                folder.id AS resource_id,
                folder.name,
                folder.path,
                folder.owner_user_id,
                owner_user.username AS owner_username,
                folder.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                'storage'::TEXT AS source_context,
                COALESCE(folder.parent_folder_id, folder.id) AS navigate_folder_id,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM folder.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
                3::INT AS access_rank
            FROM folders folder
            INNER JOIN users owner_user
                ON owner_user.id = folder.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = folder.created_by_user_id
            WHERE folder.owner_user_id = $1
              AND folder.is_deleted = false
              AND (
                  folder.name ILIKE $2
                  OR folder.path ILIKE $2
                  OR owner_user.username ILIKE $2
                  OR COALESCE(creator_user.username, 'Deleted user') ILIKE $2
              )

            UNION ALL

            SELECT
                'file'::TEXT AS resource_type,
                file_row.id AS resource_id,
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                'storage'::TEXT AS source_context,
                file_row.folder_id AS navigate_folder_id,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
                3::INT AS access_rank
            FROM files file_row
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
            WHERE file_row.owner_user_id = $1
              AND folder.owner_user_id = $1
              AND file_row.is_deleted = false
              AND folder.is_deleted = false
              AND (
                  file_row.name ILIKE $2
                  OR (
                      CASE
                          WHEN folder.path = '/' THEN '/' || file_row.name
                          ELSE folder.path || '/' || file_row.name
                      END
                  ) ILIKE $2
                  OR owner_user.username ILIKE $2
                  OR COALESCE(creator_user.username, 'Deleted user') ILIKE $2
              )

            UNION ALL

            SELECT
                'folder'::TEXT AS resource_type,
                folder.id AS resource_id,
                folder.name,
                folder.path,
                folder.owner_user_id,
                owner_user.username AS owner_username,
                folder.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                'shared'::TEXT AS source_context,
                COALESCE(folder.parent_folder_id, folder.id) AS navigate_folder_id,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM folder.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
                CASE
                    WHEN EXISTS (
                        SELECT 1
                        FROM folder_permissions folder_perm
                        INNER JOIN folders permission_folder
                            ON permission_folder.id = folder_perm.folder_id
                        WHERE folder_perm.user_id = $1
                          AND permission_folder.is_deleted = false
                          AND permission_folder.owner_user_id = folder.owner_user_id
                          AND (
                              permission_folder.path = '/'
                              OR folder.path = permission_folder.path
                              OR folder.path LIKE permission_folder.path || '/%'
                          )
                          AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                    ) THEN 2
                    WHEN EXISTS (
                        SELECT 1
                        FROM folder_permissions folder_perm
                        INNER JOIN folders permission_folder
                            ON permission_folder.id = folder_perm.folder_id
                        WHERE folder_perm.user_id = $1
                          AND permission_folder.is_deleted = false
                          AND permission_folder.owner_user_id = folder.owner_user_id
                          AND (
                              permission_folder.path = '/'
                              OR folder.path = permission_folder.path
                              OR folder.path LIKE permission_folder.path || '/%'
                          )
                          AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                    ) THEN 1
                    ELSE 0
                END AS access_rank
            FROM folders folder
            INNER JOIN users owner_user
                ON owner_user.id = folder.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = folder.created_by_user_id
            WHERE folder.owner_user_id <> $1
              AND folder.is_deleted = false
              AND (
                  folder.name ILIKE $2
                  OR folder.path ILIKE $2
                  OR owner_user.username ILIKE $2
                  OR COALESCE(creator_user.username, 'Deleted user') ILIKE $2
              )

            UNION ALL

            SELECT
                'file'::TEXT AS resource_type,
                file_row.id AS resource_id,
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                'shared'::TEXT AS source_context,
                file_row.folder_id AS navigate_folder_id,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms,
                CASE
                    WHEN EXISTS (
                        SELECT 1
                        FROM file_permissions file_perm
                        WHERE file_perm.file_id = file_row.id
                          AND file_perm.user_id = $1
                          AND lower(file_perm.privilege_type) IN ('editor', 'edit')
                    ) THEN 2
                    WHEN EXISTS (
                        SELECT 1
                        FROM folder_permissions folder_perm
                        INNER JOIN folders permission_folder
                            ON permission_folder.id = folder_perm.folder_id
                        WHERE folder_perm.user_id = $1
                          AND permission_folder.is_deleted = false
                          AND permission_folder.owner_user_id = file_row.owner_user_id
                          AND (
                              permission_folder.path = '/'
                              OR folder.path = permission_folder.path
                              OR folder.path LIKE permission_folder.path || '/%'
                          )
                          AND lower(folder_perm.privilege_type) IN ('editor', 'edit')
                    ) THEN 2
                    WHEN EXISTS (
                        SELECT 1
                        FROM file_permissions file_perm
                        WHERE file_perm.file_id = file_row.id
                          AND file_perm.user_id = $1
                          AND lower(file_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                    ) THEN 1
                    WHEN EXISTS (
                        SELECT 1
                        FROM folder_permissions folder_perm
                        INNER JOIN folders permission_folder
                            ON permission_folder.id = folder_perm.folder_id
                        WHERE folder_perm.user_id = $1
                          AND permission_folder.is_deleted = false
                          AND permission_folder.owner_user_id = file_row.owner_user_id
                          AND (
                              permission_folder.path = '/'
                              OR folder.path = permission_folder.path
                              OR folder.path LIKE permission_folder.path || '/%'
                          )
                          AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                    ) THEN 1
                    ELSE 0
                END AS access_rank
            FROM files file_row
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
            WHERE file_row.owner_user_id <> $1
              AND file_row.is_deleted = false
              AND folder.is_deleted = false
              AND (
                  file_row.name ILIKE $2
                  OR (
                      CASE
                          WHEN folder.path = '/' THEN '/' || file_row.name
                          ELSE folder.path || '/' || file_row.name
                      END
                  ) ILIKE $2
                  OR owner_user.username ILIKE $2
                  OR COALESCE(creator_user.username, 'Deleted user') ILIKE $2
              )
        ),
        ranked_entries AS (
            SELECT
                resource_type,
                resource_id,
                name,
                path,
                owner_user_id,
                owner_username,
                created_by_user_id,
                created_by_username,
                source_context,
                navigate_folder_id,
                size_bytes,
                modified_at_unix_ms,
                MAX(access_rank) AS access_rank
            FROM candidate_entries
            GROUP BY
                resource_type,
                resource_id,
                name,
                path,
                owner_user_id,
                owner_username,
                created_by_user_id,
                created_by_username,
                source_context,
                navigate_folder_id,
                size_bytes,
                modified_at_unix_ms
        )
        SELECT
            resource_type,
            resource_id,
            name,
            path,
            owner_user_id,
            owner_username,
            created_by_user_id,
            created_by_username,
            source_context,
            navigate_folder_id,
            size_bytes,
            modified_at_unix_ms,
            access_rank
        FROM ranked_entries
        WHERE access_rank > 0
        ORDER BY
            CASE WHEN source_context = 'storage' THEN 0 ELSE 1 END,
            CASE WHEN resource_type = 'folder' THEN 0 ELSE 1 END,
            LOWER(name),
            name,
            resource_id
        LIMIT $3
        OFFSET $4
        "#,
    )
    .bind(user_id)
    .bind(search_pattern)
    .bind(page_limit + 1)
    .bind(page_offset)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load search results"))?;

    let has_more = rows.len() as i64 > page_limit;
    if has_more {
        rows.truncate(page_limit as usize);
    }

    let next_cursor = if has_more {
        Some((page_offset + page_limit).to_string())
    } else {
        None
    };

    Ok((rows, next_cursor, has_more))
}

async fn load_trashed_storage_entries(
    pool: &PgPool,
    user_id: i64,
    search: Option<&str>,
) -> Result<Vec<StorageEntryRow>, ApiError> {
    sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            entries.id,
            entries.name,
            entries.path,
            entries.entry_type,
            entries.owner_user_id,
            entries.owner_username,
            entries.created_by_user_id,
            entries.created_by_username,
            entries.is_starred,
            entries.size_bytes,
            entries.modified_at_unix_ms
        FROM (
            SELECT
                folder.id,
                folder.name,
                folder.path,
                'folder'::TEXT AS entry_type,
                folder.owner_user_id,
                owner_user.username AS owner_username,
                folder.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                folder.is_starred,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM COALESCE(folder.deleted_at, folder.updated_at)) * 1000)::BIGINT AS modified_at_unix_ms
            FROM folders folder
            INNER JOIN users owner_user
                ON owner_user.id = folder.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = folder.created_by_user_id
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
                file_row.id,
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                'file'::TEXT AS entry_type,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                file_row.is_starred,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM COALESCE(file_row.deleted_at, file_row.updated_at)) * 1000)::BIGINT AS modified_at_unix_ms
            FROM files file_row
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
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

async fn load_starred_storage_entries(
    pool: &PgPool,
    user_id: i64,
    search: Option<&str>,
) -> Result<Vec<StorageEntryRow>, ApiError> {
    sqlx::query_as::<_, StorageEntryRow>(
        r#"
        SELECT
            entries.id,
            entries.name,
            entries.path,
            entries.entry_type,
            entries.owner_user_id,
            entries.owner_username,
            entries.created_by_user_id,
            entries.created_by_username,
            entries.is_starred,
            entries.size_bytes,
            entries.modified_at_unix_ms
        FROM (
            SELECT
                folder.id,
                folder.name,
                folder.path,
                'folder'::TEXT AS entry_type,
                folder.owner_user_id,
                owner_user.username AS owner_username,
                folder.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                folder.is_starred,
                NULL::BIGINT AS size_bytes,
                (EXTRACT(EPOCH FROM folder.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM folders folder
            INNER JOIN users owner_user
                ON owner_user.id = folder.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = folder.created_by_user_id
            WHERE folder.owner_user_id = $1
              AND folder.is_deleted = false
              AND folder.is_starred = true
              AND ($2::TEXT IS NULL OR folder.name ILIKE '%' || $2 || '%')

            UNION ALL

            SELECT
                file_row.id,
                file_row.name,
                CASE
                    WHEN folder.path = '/' THEN '/' || file_row.name
                    ELSE folder.path || '/' || file_row.name
                END AS path,
                'file'::TEXT AS entry_type,
                file_row.owner_user_id,
                owner_user.username AS owner_username,
                file_row.created_by_user_id,
                COALESCE(creator_user.username, 'Deleted user') AS created_by_username,
                file_row.is_starred,
                file_row.size_bytes,
                (EXTRACT(EPOCH FROM file_row.updated_at) * 1000)::BIGINT AS modified_at_unix_ms
            FROM files file_row
            INNER JOIN users owner_user
                ON owner_user.id = file_row.owner_user_id
            LEFT JOIN users creator_user
                ON creator_user.id = file_row.created_by_user_id
            INNER JOIN folders folder
                ON folder.id = file_row.folder_id
            WHERE file_row.owner_user_id = $1
              AND folder.owner_user_id = $1
              AND file_row.is_deleted = false
              AND folder.is_deleted = false
              AND file_row.is_starred = true
              AND ($2::TEXT IS NULL OR file_row.name ILIKE '%' || $2 || '%')
        ) entries
        ORDER BY entries.modified_at_unix_ms DESC, LOWER(entries.name), entries.name
        "#,
    )
    .bind(user_id)
    .bind(search)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load starred listing"))
}

async fn load_accessible_file_by_id_for_batch_download(
    pool: &PgPool,
    user_id: i64,
    file_id: i64,
) -> Result<BatchFileDownloadRow, ApiError> {
    sqlx::query_as::<_, BatchFileDownloadRow>(
        r#"
        WITH RECURSIVE ancestors AS (
            SELECT id, parent_folder_id, owner_user_id
            FROM folders
            WHERE id = (
                SELECT folder_id
                FROM files
                WHERE id = $1
                LIMIT 1
            )
              AND is_deleted = false

            UNION ALL

            SELECT parent.id, parent.parent_folder_id, parent.owner_user_id
            FROM folders parent
            INNER JOIN ancestors branch ON branch.parent_folder_id = parent.id
            WHERE parent.is_deleted = false
        )
        SELECT
            file_row.storage_path,
            CASE
                WHEN folder.path = '/' THEN '/' || file_row.name
                ELSE folder.path || '/' || file_row.name
            END AS logical_path
        FROM files file_row
        INNER JOIN folders folder
            ON folder.id = file_row.folder_id
        WHERE file_row.id = $1
          AND file_row.is_deleted = false
          AND folder.is_deleted = false
          AND (
                file_row.owner_user_id = $2
                OR EXISTS (
                    SELECT 1
                    FROM ancestors branch
                    WHERE branch.owner_user_id = $2
                )
                OR EXISTS (
                    SELECT 1
                    FROM file_permissions file_perm
                    WHERE file_perm.file_id = file_row.id
                      AND file_perm.user_id = $2
                      AND lower(file_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                )
                OR EXISTS (
                    SELECT 1
                    FROM folder_permissions folder_perm
                    INNER JOIN ancestors branch ON branch.id = folder_perm.folder_id
                    WHERE folder_perm.user_id = $2
                      AND lower(folder_perm.privilege_type) IN ('viewer', 'view', 'read', 'editor', 'edit')
                )
          )
        LIMIT 1
        "#,
    )
    .bind(file_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to resolve batch file download target"))?
    .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned()))
}

async fn load_folder_files_for_batch_download(
    pool: &PgPool,
    owner_user_id: i64,
    folder_path: &str,
) -> Result<Vec<BatchFileDownloadRow>, ApiError> {
    let folder_prefix = format!("{}/%", folder_path.trim_end_matches('/'));

    sqlx::query_as::<_, BatchFileDownloadRow>(
        r#"
        SELECT
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
              folder.path = $2
              OR folder.path LIKE $3
          )
        ORDER BY
            LENGTH(folder.path),
            LOWER(folder.path),
            LOWER(file_row.name),
            file_row.id
        "#,
    )
    .bind(owner_user_id)
    .bind(folder_path)
    .bind(folder_prefix)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load folder files for batch download"))
}

fn folder_download_label(folder_path: &str) -> String {
    let normalized = normalize_db_path(folder_path);
    if normalized == "/" {
        return "My Storage".to_owned();
    }

    normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .next_back()
        .map(str::to_owned)
        .unwrap_or_else(|| "folder".to_owned())
}

fn file_download_label(file_path: &str) -> String {
    let normalized = normalize_db_path(file_path);

    normalized
        .split('/')
        .filter(|segment| !segment.is_empty())
        .next_back()
        .map(str::to_owned)
        .unwrap_or_else(|| "file".to_owned())
}

fn strip_folder_prefix(child_path: &str, folder_path: &str) -> String {
    let normalized_child = normalize_db_path(child_path);
    let normalized_folder = normalize_db_path(folder_path);

    if normalized_folder == "/" {
        return normalized_child.trim_start_matches('/').to_owned();
    }

    if normalized_child == normalized_folder {
        return String::new();
    }

    let prefix = format!("{}/", normalized_folder.trim_end_matches('/'));
    if let Some(stripped) = normalized_child.strip_prefix(&prefix) {
        return stripped.to_owned();
    }

    normalized_child.trim_start_matches('/').to_owned()
}

fn normalize_zip_entry_path(raw_path: &str) -> String {
    let mut segments: Vec<String> = Vec::new();

    for segment in raw_path.split(['/', '\\']) {
        let trimmed = segment.trim();

        if trimmed.is_empty() || trimmed == "." {
            continue;
        }

        if trimmed == ".." {
            continue;
        }

        let clean: String = trimmed.chars().filter(|ch| !ch.is_control()).collect();
        if clean.is_empty() {
            continue;
        }

        segments.push(clean);
    }

    segments.join("/")
}

fn make_unique_zip_entry_path(path: &str, used_paths: &mut HashSet<String>) -> String {
    if used_paths.insert(path.to_owned()) {
        return path.to_owned();
    }

    let (parent, name) = if let Some((path_parent, file_name)) = path.rsplit_once('/') {
        (path_parent, file_name)
    } else {
        ("", path)
    };

    let extension_split = name.rfind('.').filter(|index| *index > 0 && *index < name.len() - 1);
    let (stem, extension) = if let Some(index) = extension_split {
        (&name[..index], Some(&name[index + 1..]))
    } else {
        (name, None)
    };

    let base_stem = if stem.trim().is_empty() { "file" } else { stem };
    let mut suffix_index = 2;

    loop {
        let candidate_name = match extension {
            Some(ext) => format!("{base_stem} ({suffix_index}).{ext}"),
            None => format!("{base_stem} ({suffix_index})"),
        };
        let candidate_path = if parent.is_empty() {
            candidate_name
        } else {
            format!("{parent}/{candidate_name}")
        };

        if used_paths.insert(candidate_path.clone()) {
            return candidate_path;
        }

        suffix_index += 1;
    }
}

fn storage_entry_row_to_dto(row: StorageEntryRow) -> StorageEntryDto {
    StorageEntryDto {
        id: row.id,
        name: row.name,
        path: normalize_db_path(&row.path),
        entry_type: row.entry_type,
        owner_user_id: row.owner_user_id,
        owner_username: row.owner_username,
        created_by_user_id: row.created_by_user_id,
        created_by_username: row.created_by_username,
        is_starred: row.is_starred,
        size_bytes: row.size_bytes,
        modified_at_unix_ms: Some(row.modified_at_unix_ms),
    }
}

fn is_descendant_path(path: &str, ancestor: &str) -> bool {
    let normalized_path = normalize_db_path(path);
    let normalized_ancestor = normalize_db_path(ancestor);

    if normalized_ancestor == "/" {
        return normalized_path != "/";
    }

    normalized_path.len() > normalized_ancestor.len()
        && normalized_path.starts_with(&normalized_ancestor)
        && normalized_path
            .as_bytes()
            .get(normalized_ancestor.len())
            .copied()
            == Some(b'/')
}

fn normalize_storage_list_limit(raw: Option<i64>) -> i64 {
    let requested = raw.unwrap_or(DEFAULT_STORAGE_LIST_LIMIT);
    requested.clamp(1, MAX_STORAGE_LIST_LIMIT)
}

fn normalize_search_list_limit(raw: Option<i64>) -> i64 {
    let requested = raw.unwrap_or(DEFAULT_SEARCH_LIST_LIMIT);
    requested.clamp(1, MAX_SEARCH_LIST_LIMIT)
}

fn parse_storage_list_cursor(raw: Option<&str>) -> Result<i64, ApiError> {
    let Some(value) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(0);
    };

    let parsed = value
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("Invalid storage list cursor".to_owned()))?;

    if parsed < 0 {
        return Err(ApiError::BadRequest(
            "Invalid storage list cursor".to_owned(),
        ));
    }

    Ok(parsed)
}

fn access_level_from_rank(rank: i32) -> Option<AccessLevel> {
    if rank >= 3 {
        return Some(AccessLevel::Owner);
    }

    if rank >= 2 {
        return Some(AccessLevel::Editor);
    }

    if rank >= 1 {
        return Some(AccessLevel::Viewer);
    }

    None
}

async fn lookup_username_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: i64,
) -> Result<String, ApiError> {
    sqlx::query_scalar::<_, String>(
        r#"
        SELECT username
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load username"))?
    .ok_or_else(|| ApiError::BadRequest("User account does not exist".to_owned()))
}

async fn lookup_optional_username_by_id_tx(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Option<i64>,
) -> Result<String, ApiError> {
    let Some(user_id) = user_id else {
        return Ok("Deleted user".to_owned());
    };

    let username = sqlx::query_scalar::<_, String>(
        r#"
        SELECT username
        FROM users
        WHERE id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| ApiError::internal_with_context("Failed to load username"))?;

    Ok(username.unwrap_or_else(|| "Deleted user".to_owned()))
}

async fn load_resource_owner(
    pool: &PgPool,
    resource_type: StorageEntryKind,
    resource_id: i64,
) -> Result<ResourceOwnerRow, ApiError> {
    match resource_type {
        StorageEntryKind::Folder => sqlx::query_as::<_, ResourceOwnerRow>(
            r#"
            SELECT owner_user_id, name AS resource_name
            FROM folders
            WHERE id = $1
              AND is_deleted = false
            LIMIT 1
            "#,
        )
        .bind(resource_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to resolve folder owner"))?
        .ok_or_else(|| ApiError::BadRequest("Requested folder does not exist".to_owned())),
        StorageEntryKind::File => sqlx::query_as::<_, ResourceOwnerRow>(
            r#"
            SELECT owner_user_id, name AS resource_name
            FROM files
            WHERE id = $1
              AND is_deleted = false
            LIMIT 1
            "#,
        )
        .bind(resource_id)
        .fetch_optional(pool)
        .await
        .map_err(|_| ApiError::internal_with_context("Failed to resolve file owner"))?
        .ok_or_else(|| ApiError::BadRequest("Requested file does not exist".to_owned())),
    }
}

fn normalize_share_privilege(raw: &str) -> Result<&'static str, ApiError> {
    let normalized = raw.trim().to_ascii_lowercase();

    match normalized.as_str() {
        "viewer" | "view" | "read" => Ok("viewer"),
        "editor" | "edit" => Ok("editor"),
        _ => Err(ApiError::BadRequest(
            "privilegeType must be either 'viewer' or 'editor'".to_owned(),
        )),
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

fn logical_path_to_storage_prefix(user_id: i64, logical_path: &str) -> String {
    let relative = logical_path_to_relative_path(logical_path);
    let storage_rel = Path::new("users").join(user_id.to_string()).join(relative);
    format!("/{}", storage_rel.to_string_lossy().replace('\\', "/"))
}

fn rollback_filesystem_moves(applied_moves: &[(PathBuf, PathBuf)]) {
    for (from_path, to_path) in applied_moves.iter().rev() {
        let _ = fs::rename(to_path, from_path);
    }
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
