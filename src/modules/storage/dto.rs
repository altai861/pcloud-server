use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageEntryDto {
    pub name: String,
    pub path: String,
    pub entry_type: String,
    pub size_bytes: Option<i64>,
    pub modified_at_unix_ms: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageListResponse {
    pub current_path: String,
    pub parent_path: Option<String>,
    pub entries: Vec<StorageEntryDto>,
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
    pub created_at_unix_ms: i64,
    pub modified_at_unix_ms: i64,
    pub folder_count: i64,
    pub file_count: i64,
    pub total_item_count: i64,
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
