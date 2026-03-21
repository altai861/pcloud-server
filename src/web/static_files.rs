use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist/admin-launch"]
struct AdminAssets;

#[derive(RustEmbed)]
#[folder = "web/dist/main-app"]
struct ClientAssets;

pub async fn serve_admin_static(path: &str) -> Response {
    serve_static::<AdminAssets>(path)
}

pub async fn serve_client_static(path: &str) -> Response {
    serve_static::<ClientAssets>(path)
}

fn serve_static<T: RustEmbed>(request_path: &str) -> Response {
    let normalized = normalize_path(request_path);
    let fallback_to_index = !normalized.contains('.');
    let mime = from_path(&normalized).first_or_octet_stream();

    if let Some(content) = T::get(&normalized) {
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data,
        )
            .into_response();
    }

    if fallback_to_index {
        if let Some(content) = T::get("index.html") {
            return (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
                content.data,
            )
                .into_response();
        }
    }

    (StatusCode::NOT_FOUND, "Not Found").into_response()
}

fn normalize_path(path: &str) -> String {
    let trimmed = path.trim_start_matches('/').trim();

    if trimmed.is_empty() {
        "index.html".to_owned()
    } else {
        trimmed.to_owned()
    }
}
