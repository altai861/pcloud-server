use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::setup::{
        dto::{SetupInitializeRequest, SetupInitializeResponse, SetupStatusResponse},
        service,
    },
};
use axum::{Json, extract::State};

pub async fn status(State(state): State<AppState>) -> ApiResult<Json<SetupStatusResponse>> {
    let is_initialized = service::is_initialized(&state.pool).await?;

    Ok(Json(SetupStatusResponse { is_initialized }))
}

pub async fn initialize(
    State(state): State<AppState>,
    Json(payload): Json<SetupInitializeRequest>,
) -> ApiResult<Json<SetupInitializeResponse>> {
    service::initialize(&state.pool, payload).await?;

    Ok(Json(SetupInitializeResponse {
        is_initialized: true,
        message: "Initial system setup completed successfully".to_owned(),
    }))
}
