use crate::{
    app_state::AppState,
    error::ApiResult,
    modules::auth::{
        dto::{
            LoginRequest, LoginResponse, LogoutResponse, MeResponse, UpdateProfileImageResponse,
        },
        service,
    },
};
use axum::{
    Json,
    body::Body,
    extract::{Multipart, State},
    http::{HeaderMap, header},
    response::Response,
};

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

pub async fn update_profile_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> ApiResult<Json<UpdateProfileImageResponse>> {
    let current = service::authenticate_headers(&state.pool, &headers).await?;

    let mut image_bytes: Option<Vec<u8>> = None;
    let mut content_type: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| crate::error::ApiError::BadRequest("Invalid multipart payload".to_owned()))?
    {
        if field.name() != Some("image") {
            continue;
        }

        content_type = field.content_type().map(|value| value.to_owned());
        image_bytes = Some(
            field
                .bytes()
                .await
                .map_err(|_| {
                    crate::error::ApiError::BadRequest("Invalid image payload".to_owned())
                })?
                .to_vec(),
        );
        break;
    }

    let image_bytes = image_bytes
        .ok_or_else(|| crate::error::ApiError::BadRequest("Missing image file field".to_owned()))?;
    let content_type = content_type.ok_or_else(|| {
        crate::error::ApiError::BadRequest("Missing image content-type".to_owned())
    })?;

    let updated_user =
        service::update_profile_image(&state.pool, &current, &image_bytes, &content_type).await?;

    Ok(Json(UpdateProfileImageResponse {
        message: "Profile image updated successfully".to_owned(),
        user: updated_user,
    }))
}

pub async fn profile_image(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    let current = service::authenticate_headers(&state.pool, &headers).await?;
    let (bytes, content_type) = service::read_profile_image(&state.pool, &current).await?;

    Ok(Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .map_err(|_| {
            crate::error::ApiError::internal_with_context("Failed to build image response")
        })?)
}
