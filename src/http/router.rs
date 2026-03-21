use crate::{
    app_state::AppState,
    modules::{
        auth::handlers as auth_handlers, setup::handlers as setup_handlers,
        storage::handlers as storage_handlers, system::handlers as system_handlers,
    },
    web::static_files::{serve_admin_static, serve_client_static},
};
use axum::{
    Router,
    http::Uri,
    response::Response,
    routing::{get, post},
};

pub fn build_client_router(state: AppState) -> Router {
    Router::new()
        .route("/api/client/status", get(system_handlers::server_status))
        .route("/api/client/auth/login", post(auth_handlers::login))
        .route("/api/client/auth/logout", post(auth_handlers::logout))
        .route("/api/client/me", get(auth_handlers::me))
        .route("/api/client/storage/list", get(storage_handlers::list))
        .route("/api/setup/status", get(setup_handlers::status))
        .route("/", get(client_index_handler))
        .route("/*file", get(client_static_handler))
        .with_state(state)
}

pub fn build_admin_router(state: AppState) -> Router {
    Router::new()
        .route("/api/setup/status", get(setup_handlers::status))
        .route("/api/setup/initialize", post(setup_handlers::initialize))
        .route("/", get(admin_index_handler))
        .route("/*file", get(admin_static_handler))
        .with_state(state)
}

async fn admin_index_handler() -> Response {
    serve_admin_static("index.html").await
}

async fn admin_static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    serve_admin_static(path).await
}

async fn client_index_handler() -> Response {
    serve_client_static("index.html").await
}

async fn client_static_handler(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    serve_client_static(path).await
}
