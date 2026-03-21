use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::{
        auth::service,
        storage::{
            dto::StorageListResponse,
            service::{self as storage_service, StorageListQuery},
        },
    },
};
use axum::{
    Json,
    extract::{Query, State},
    http::HeaderMap,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListStorageRequest {
    pub path: Option<String>,
    pub q: Option<String>,
}

pub async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ListStorageRequest>,
) -> ApiResult<Json<StorageListResponse>> {
    let current_user = service::authenticate_headers(&state.pool, &headers).await?;

    let result = storage_service::list_user_storage(
        &state.pool,
        &current_user,
        StorageListQuery {
            path: query.path,
            search: query.q,
        },
    )
    .await?;

    Ok(Json(StorageListResponse {
        current_path: result.current_path,
        parent_path: result.parent_path,
        entries: result.entries,
        total_storage_limit_bytes: result.total_storage_limit_bytes,
        total_storage_used_bytes: result.total_storage_used_bytes,
        user_storage_quota_bytes: result.user_storage_quota_bytes,
        user_storage_used_bytes: result.user_storage_used_bytes,
    }))
}
