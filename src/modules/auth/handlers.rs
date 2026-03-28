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
    extract::{Multipart, Query, State, multipart::MultipartError},
    http::{
        HeaderMap, StatusCode,
        header::{self, AUTHORIZATION},
    },
    response::Response,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfileImageRequest {
    pub user_id: i64,
    pub access_token: Option<String>,
}

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
        .map_err(map_profile_image_multipart_error)?
    {
        if field.name() != Some("image") {
            continue;
        }

        content_type = field.content_type().map(|value| value.to_owned());
        image_bytes = Some(
            field
                .bytes()
                .await
                .map_err(map_profile_image_multipart_error)?
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

fn map_profile_image_multipart_error(err: MultipartError) -> crate::error::ApiError {
    if err.status() == StatusCode::PAYLOAD_TOO_LARGE {
        return crate::error::ApiError::BadRequest(
            "Profile image limit exceeded. Maximum size is 30 MB".to_owned(),
        );
    }

    crate::error::ApiError::BadRequest(format!("Invalid image payload: {}", err.body_text()))
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

pub async fn user_profile_image(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<UserProfileImageRequest>,
) -> ApiResult<Response> {
    if headers.get(AUTHORIZATION).is_some() {
        let _ = service::authenticate_headers(&state.pool, &headers).await?;
    } else if let Some(token) = query.access_token.as_deref() {
        let _ = service::authenticate_access_token(&state.pool, token).await?;
    } else {
        return Err(crate::error::ApiError::Unauthorized(
            "Missing access token".to_owned(),
        ));
    }

    let (bytes, content_type) =
        service::read_user_profile_image(&state.pool, query.user_id).await?;

    Ok(Response::builder()
        .status(200)
        .header(header::CONTENT_TYPE, content_type)
        .body(Body::from(bytes))
        .map_err(|_| {
            crate::error::ApiError::internal_with_context("Failed to build image response")
        })?)
}
