use axum::{Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct Status {
    pub status: String
}

pub async fn server_status() -> Json<Status> {

    Json(Status {
        status: "server running".to_string()
    })
}