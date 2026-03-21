use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::auth::{
        dto::{LoginRequest, LoginResponse, LogoutResponse, MeResponse},
        service,
    },
};
use axum::{Json, extract::State, http::HeaderMap};

pub async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<LoginRequest>,
) -> ApiResult<Json<LoginResponse>> {
    let response = service::login(&state.pool, payload, &headers).await?;

    Ok(Json(response))
}

pub async fn me(State(state): State<AppState>, headers: HeaderMap) -> ApiResult<Json<MeResponse>> {
    let current = service::authenticate_headers(&state.pool, &headers).await?;

    Ok(Json(MeResponse { user: current.user }))
}

pub async fn logout(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Json<LogoutResponse>> {
    let current = service::authenticate_headers(&state.pool, &headers).await?;
    service::revoke_current_session(&state.pool, current.session_id).await?;

    Ok(Json(LogoutResponse {
        message: "Signed out successfully".to_owned(),
    }))
}
