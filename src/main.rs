mod api {
    pub mod client;
    pub mod admin;
}

mod web {
    pub mod static_files;
}

mod network {
    pub mod mdns;
}

use axum::{
    response::Response,
    routing::get,
    Router,
};
use web::static_files::serve_static;
use network::mdns::start_mdns_service;

#[tokio::main]
async fn main() {
    let _mdns = start_mdns_service(8080);

    let app = Router::new()
        .route("/api/client/status", get(api::client::server_status))
        .route("/api/admin/pairings", get(api::admin::list_pairings))
        .route("/", get(index_handler))
        .route("/*file", get(handler));

    println!("Server running on http://localhost:8080");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}

async fn index_handler() -> Response {
    serve_static("index.html".to_string()).await
}

async fn handler(uri: axum::http::Uri) -> Response {
    println!("{}", uri.path());
    let path = uri.path().trim_start_matches('/').to_string();
    serve_static(path).await
}