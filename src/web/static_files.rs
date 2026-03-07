use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
pub struct WebAssets;

use axum::{
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use mime_guess::from_path;

pub async fn serve_static(path: String) -> Response {
    println!("{}", path);
    let path = if path.is_empty() {
        "index.html"
    } else {
        &path
    };

    match WebAssets::get(path) {

        Some(content) => {

            let mime = from_path(path).first_or_octet_stream();

            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, mime.as_ref())],
                content.data,
            ).into_response()

        }

        None => (StatusCode::NOT_FOUND, "Not Found").into_response()
    }
}