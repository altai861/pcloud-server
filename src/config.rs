use std::{env, net::SocketAddr};

#[derive(Clone, Debug)]
pub struct Config {
    pub database_url: String,
    pub client_bind: SocketAddr,
    pub admin_bind: SocketAddr,
    pub mode: AppMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AppMode {
    Dev,
    Prod,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        // Load .env if present so local development works with `cargo run`.
        let _ = dotenvy::dotenv();

        let database_url =
            env::var("DATABASE_URL").map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        let client_bind = env::var("PCLOUD_CLIENT_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
            .parse::<SocketAddr>()
            .map_err(|e| anyhow::anyhow!("Invalid PCLOUD_CLIENT_BIND: {e}"))?;

        let admin_bind = env::var("PCLOUD_ADMIN_BIND")
            .unwrap_or_else(|_| "127.0.0.1:9090".to_owned())
            .parse::<SocketAddr>()
            .map_err(|e| anyhow::anyhow!("Invalid PCLOUD_ADMIN_BIND: {e}"))?;

        let mode = match env::var("PCLOUD_MODE")
            .unwrap_or_else(|_| "PROD".to_owned())
            .trim()
            .to_ascii_uppercase()
            .as_str()
        {
            "DEV" => AppMode::Dev,
            "PROD" => AppMode::Prod,
            other => {
                return Err(anyhow::anyhow!(
                    "Invalid PCLOUD_MODE: {other}. Expected DEV or PROD"
                ));
            }
        };

        Ok(Self {
            database_url,
            client_bind,
            admin_bind,
            mode,
        })
    }
}
