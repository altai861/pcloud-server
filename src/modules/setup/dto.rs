use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInitializeRequest {
    pub admin: AdminSetupRequest,
    pub system: SystemSetupRequest,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminSetupRequest {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub password: String,
    pub password_confirmation: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SystemSetupRequest {
    pub storage_root_path: String,
    pub total_storage_limit_bytes: Option<i64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupStatusResponse {
    pub is_initialized: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SetupInitializeResponse {
    pub is_initialized: bool,
    pub message: String,
}
