use crate::{app_state::AppState, error::ApiResult, modules::setup::service};
use axum::{Json, extract::State};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerStatusResponse {
    pub status: String,
    pub is_initialized: bool,
}

pub async fn server_status(State(state): State<AppState>) -> ApiResult<Json<ServerStatusResponse>> {
    let initialized = service::is_initialized(&state.pool).await?;

    Ok(Json(ServerStatusResponse {
        status: "server running".to_owned(),
        is_initialized: initialized,
    }))
}
