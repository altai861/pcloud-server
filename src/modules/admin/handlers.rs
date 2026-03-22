use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::{
        admin::{
            dto::{
                AdminCreateUserResponse, AdminDeleteUserResponse, AdminUpdateUserResponse,
                AdminUserListResponse,
            },
            service::{self as admin_service, CreateUserInput, UpdateUserInput},
        },
        auth::service,
    },
};
use axum::{
    Json,
    extract::{Path, State},
    http::HeaderMap,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub password: String,
    pub password_confirmation: String,
    pub storage_quota_bytes: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub username: String,
    pub email: String,
    pub full_name: String,
    pub storage_quota_bytes: i64,
}

pub async fn list_users(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<AdminUserListResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;
    let users = admin_service::list_users(&state.pool, &current_user).await?;

    Ok(Json(AdminUserListResponse { users }))
}

pub async fn create_user(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateUserRequest>,
) -> ApiResult<Json<AdminCreateUserResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let created_user = admin_service::create_user(
        &state.pool,
        &current_user,
        CreateUserInput {
            username: payload.username,
            email: payload.email,
            full_name: payload.full_name,
            password: payload.password,
            password_confirmation: payload.password_confirmation,
            storage_quota_bytes: payload.storage_quota_bytes,
        },
    )
    .await?;

    Ok(Json(AdminCreateUserResponse {
        message: "User created successfully".to_owned(),
        user: created_user,
    }))
}

pub async fn update_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
    Json(payload): Json<UpdateUserRequest>,
) -> ApiResult<Json<AdminUpdateUserResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let updated_user = admin_service::update_user(
        &state.pool,
        &current_user,
        user_id,
        UpdateUserInput {
            username: payload.username,
            email: payload.email,
            full_name: payload.full_name,
            storage_quota_bytes: payload.storage_quota_bytes,
        },
    )
    .await?;

    Ok(Json(AdminUpdateUserResponse {
        message: "User updated successfully".to_owned(),
        user: updated_user,
    }))
}

pub async fn delete_user(
    State(state): State<AppState>,
    Path(user_id): Path<i64>,
    headers: HeaderMap,
) -> ApiResult<Json<AdminDeleteUserResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    admin_service::delete_user(&state.pool, &current_user, user_id).await?;

    Ok(Json(AdminDeleteUserResponse {
        message: "User deleted successfully".to_owned(),
    }))
}
