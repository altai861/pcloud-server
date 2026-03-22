use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageEntryDto {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub owner_user_id: i64,
    pub owner_username: String,
    pub created_by_user_id: Option<i64>,
    pub created_by_username: String,
    pub is_starred: bool,
    pub size_bytes: Option<i64>,
    pub modified_at_unix_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageListResponse {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMutationResponse {
    pub message: String,
    pub entry: StorageEntryDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageFolderMetadataResponse {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageFileMetadataResponse {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageDeleteResponse {
    pub message: String,
    pub deleted_path: String,
    pub entry_type: String,
    pub reclaimed_bytes: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageRestoreResponse {
    pub message: String,
    pub restored_path: String,
    pub entry_type: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageMoveResponse {
    pub message: String,
    pub moved_count: i64,
    pub destination_folder_id: i64,
    pub destination_path: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedResourceEntryDto {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedResourcesListResponse {
    pub entries: Vec<SharedResourceEntryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResourceEntryDto {
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResourcesResponse {
    pub query: String,
    pub entries: Vec<SearchResourceEntryDto>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedPermissionTargetDto {
    pub user_id: i64,
    pub username: String,
    pub full_name: String,
    pub privilege_type: String,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SharedPermissionsResponse {
    pub resource_type: String,
    pub resource_id: i64,
    pub resource_name: String,
    pub entries: Vec<SharedPermissionTargetDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareableUserDto {
    pub user_id: i64,
    pub username: String,
    pub full_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareableUsersResponse {
    pub users: Vec<ShareableUserDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShareMutationResponse {
    pub message: String,
}
