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

    let client_app = Router::new()
        .route("/api/client/status", get(api::client::server_status));

    let admin_app = Router::new()
        .route("/api/admin/pairings", get(api::admin::list_pairings))
        .route("/", get(index_handler))
        .route("/*file", get(handler));

    let client_listener = tokio::net::TcpListener::bind("0.0.0.0:8080")
        .await
        .unwrap();

    let admin_listener = tokio::net::TcpListener::bind("127.0.0.1:9090")
        .await
        .unwrap();

    println!("Client API running on http://0.0.0.0:8080");
    println!("Admin UI running on http://127.0.0.1:9090");

    let client_server = axum::serve(client_listener, client_app);
    let admin_server = axum::serve(admin_listener, admin_app);

    let (client_result, admin_result) = tokio::join!(client_server, admin_server);

    client_result.unwrap();
    admin_result.unwrap();
}

async fn index_handler() -> Response {
    serve_static("index.html".to_string()).await
}

async fn handler(uri: axum::http::Uri) -> Response {
    println!("{}", uri.path());
    let path = uri.path().trim_start_matches('/').to_string();
    serve_static(path).await
}