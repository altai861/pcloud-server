mod app_state;
mod config;
mod db;
mod error;
mod http;
mod modules;
mod web;

use crate::app_state::AppState;
use crate::config::Config;
use crate::db::{connect_pool, run_migrations};
use crate::http::router::{build_admin_router, build_client_router};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let pool = connect_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let state = AppState::new(pool);

    let client_app = build_client_router(state.clone());
    let admin_app = build_admin_router(state);

    let client_listener = tokio::net::TcpListener::bind(config.client_bind).await?;
    let admin_listener = tokio::net::TcpListener::bind(config.admin_bind).await?;

    println!("Client API + Web App: http://{}", config.client_bind);
    println!("Admin Setup API + Web App: http://{}", config.admin_bind);

    let client_server = axum::serve(client_listener, client_app);
    let admin_server = axum::serve(admin_listener, admin_app);

    let (client_result, admin_result) = tokio::join!(client_server, admin_server);

    client_result?;
    admin_result?;

    Ok(())
}
