use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserDto {
    pub id: i64,
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub role: String,
    pub status: String,
    pub storage_quota_bytes: i64,
    pub storage_used_bytes: i64,
    pub created_at_unix_ms: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUserListResponse {
    pub users: Vec<AdminUserDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminCreateUserResponse {
    pub message: String,
    pub user: AdminUserDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminUpdateUserResponse {
    pub message: String,
    pub user: AdminUserDto,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminDeleteUserResponse {
    pub message: String,
}
