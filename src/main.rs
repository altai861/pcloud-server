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
use crate::modules::setup::service::is_initialized;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let pool = connect_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let state = AppState::new(pool);
    let initialized = is_initialized(&state.pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to determine setup state: {error:?}"))?;

    let client_app = build_client_router(state.clone());
    let client_listener = tokio::net::TcpListener::bind(config.client_bind).await?;
    println!("Client API + Web App: http://{}", config.client_bind);
    let client_server = axum::serve(client_listener, client_app);

    if initialized {
        println!("Admin setup web is disabled because system is already initialized.");
        client_server.await?;
        return Ok(());
    }

    let admin_app = build_admin_router(state);
    let admin_listener = tokio::net::TcpListener::bind(config.admin_bind).await?;
    println!("Admin Setup API + Web App: http://{}", config.admin_bind);

    let admin_server = axum::serve(admin_listener, admin_app);

    let (client_result, admin_result) = tokio::join!(client_server, admin_server);

    client_result?;
    admin_result?;

    Ok(())
}
