mod app_state;
mod config;
mod db;
mod discovery;
mod error;
mod http;
mod modules;
mod relay;
mod web;

use crate::app_state::AppState;
use crate::config::{AppMode, Config};
use crate::db::{connect_pool, run_migrations};
use crate::discovery::{MdnsDiscoveryConfig, run_mdns_discovery};
use crate::http::router::{build_admin_router, build_client_router};
use crate::modules::setup::service::is_initialized;
use crate::relay::{client::run_relay_client, config::RelayClientConfig};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env()?;
    let pool = connect_pool(&config.database_url).await?;
    run_migrations(&pool).await?;

    let state = AppState::new(pool);
    let initialized = is_initialized(&state.pool)
        .await
        .map_err(|error| anyhow::anyhow!("Failed to determine setup state: {error:?}"))?;

    let request_logging_enabled = config.mode == AppMode::Dev;

    if request_logging_enabled {
        println!("DEV mode request logging is enabled.");
    }

    let client_app = build_client_router(state.clone(), request_logging_enabled);
    let client_listener = tokio::net::TcpListener::bind(config.client_bind).await?;
    println!("Client API + Web App: http://{}", config.client_bind);
    let client_server = axum::serve(client_listener, client_app);

    let relay_config = RelayClientConfig::from_env(config.client_bind)?;

    if let Some(mdns_config) =
        MdnsDiscoveryConfig::from_env(config.client_bind, relay_config.as_ref())
    {
        tokio::task::spawn_blocking(move || run_mdns_discovery(mdns_config));
    }

    if let Some(relay_config) = relay_config {
        println!(
            "Relay client enabled. Device ID: {}. Local target: {}",
            relay_config.device_id, relay_config.local_base_url
        );
        tokio::spawn(run_relay_client(relay_config));
    }

    if initialized {
        println!("Admin setup web is disabled because system is already initialized.");
        client_server.await?;
        return Ok(());
    }

    let admin_app = build_admin_router(state, request_logging_enabled);
    let admin_listener = tokio::net::TcpListener::bind(config.admin_bind).await?;
    println!("Admin Setup API + Web App: http://{}", config.admin_bind);

    let admin_server = axum::serve(admin_listener, admin_app);

    let (client_result, admin_result) = tokio::join!(client_server, admin_server);

    client_result?;
    admin_result?;

    Ok(())
}
