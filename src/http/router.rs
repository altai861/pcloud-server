use crate::{
    app_state::AppState,
    modules::{
        admin::handlers as admin_handlers, auth::handlers as auth_handlers,
        setup::handlers as setup_handlers, storage::handlers as storage_handlers,
        system::handlers as system_handlers,
    },
    web::static_files::{serve_admin_static, serve_client_static},
};
use axum::{
    Router,
    extract::DefaultBodyLimit,
    http::Uri,
    response::Response,
    routing::{delete, get, post, put},
};

const MAX_UPLOAD_REQUEST_BYTES: usize = 5 * 1024 * 1024 * 1024;

pub fn build_client_router(state: AppState) -> Router {
    Router::new()
        .route("/api/client/status", get(system_handlers::server_status))
        .route("/api/client/auth/login", post(auth_handlers::login))
        .route("/api/client/auth/logout", post(auth_handlers::logout))
        .route("/api/client/me", get(auth_handlers::me))
        .route(
            "/api/client/me/profile-image",
            get(auth_handlers::profile_image).post(auth_handlers::update_profile_image),
        )
        .route(
            "/api/client/users/profile-image",
            get(auth_handlers::user_profile_image),
        )
        .route("/api/client/storage/list", get(storage_handlers::list))
        .route(
            "/api/client/storage/trash/list",
            get(storage_handlers::list_trash),
        )
        .route(
            "/api/client/storage/starred/list",
            get(storage_handlers::list_starred),
        )
        .route(
            "/api/client/storage/shared/list",
            get(storage_handlers::list_shared),
        )
        .route(
            "/api/client/storage/shares/users",
            get(storage_handlers::search_shareable_users),
        )
        .route(
            "/api/client/storage/shares",
            get(storage_handlers::list_share_permissions)
                .put(storage_handlers::upsert_share_permission)
                .delete(storage_handlers::remove_share_permission),
        )
        .route(
            "/api/client/storage/starred",
            put(storage_handlers::set_starred),
        )
        .route(
            "/api/client/storage/folders/metadata",
            get(storage_handlers::folder_metadata),
        )
        .route(
            "/api/client/storage/folders",
            post(storage_handlers::create_folder)
                .put(storage_handlers::rename_folder)
                .delete(storage_handlers::delete_folder),
        )
        .route(
            "/api/client/storage/files/upload",
            post(storage_handlers::upload_file)
                .layer(DefaultBodyLimit::max(MAX_UPLOAD_REQUEST_BYTES)),
        )
        .route(
            "/api/client/storage/files",
            put(storage_handlers::rename_file).delete(storage_handlers::delete_file),
        )
        .route(
            "/api/client/storage/files/metadata",
            get(storage_handlers::file_metadata),
        )
        .route(
            "/api/client/storage/trash/folders",
            delete(storage_handlers::permanently_delete_folder),
        )
        .route(
            "/api/client/storage/trash/files",
            delete(storage_handlers::permanently_delete_file),
        )
        .route(
            "/api/client/storage/trash/folders/restore",
            post(storage_handlers::restore_folder),
        )
        .route(
            "/api/client/storage/trash/files/restore",
            post(storage_handlers::restore_file),
        )
        .route(
            "/api/client/storage/files/download",
            get(storage_handlers::download_file),
        )
        .route(
            "/api/client/admin/users",
            get(admin_handlers::list_users).post(admin_handlers::create_user),
        )
        .route(
            "/api/client/admin/users/:user_id",
            put(admin_handlers::update_user).delete(admin_handlers::delete_user),
        )
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
