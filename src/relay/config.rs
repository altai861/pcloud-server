use std::{env, net::SocketAddr, time::Duration};

const DEFAULT_RELAY_URL: &str = "ws://127.0.0.1:7070/api/relay/device/connect";
const DEFAULT_RECONNECT_SECONDS: u64 = 5;
const DEFAULT_LOCAL_REQUEST_TIMEOUT_SECONDS: u64 = 60;

#[derive(Clone, Debug)]
pub struct RelayClientConfig {
    pub relay_url: String,
    pub device_id: String,
    pub token: String,
    pub local_base_url: String,
    pub reconnect_delay: Duration,
    pub local_request_timeout: Duration,
}

impl RelayClientConfig {
    pub fn from_env(client_bind: SocketAddr) -> anyhow::Result<Option<Self>> {
        if !relay_enabled() {
            return Ok(None);
        }

        let relay_url =
            env::var("PCLOUD_RELAY_URL").unwrap_or_else(|_| DEFAULT_RELAY_URL.to_owned());
        let device_id = required_env("PCLOUD_DEVICE_ID")?;
        let token = required_env("PCLOUD_RELAY_TOKEN")?;
        let local_base_url = env::var("PCLOUD_RELAY_LOCAL_BASE_URL")
            .unwrap_or_else(|_| format!("http://127.0.0.1:{}", client_bind.port()));

        let reconnect_delay = env::var("PCLOUD_RELAY_RECONNECT_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_RECONNECT_SECONDS);

        let local_request_timeout = env::var("PCLOUD_RELAY_LOCAL_REQUEST_TIMEOUT_SECONDS")
            .ok()
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(DEFAULT_LOCAL_REQUEST_TIMEOUT_SECONDS);

        Ok(Some(Self {
            relay_url,
            device_id,
            token,
            local_base_url,
            reconnect_delay: Duration::from_secs(reconnect_delay),
            local_request_timeout: Duration::from_secs(local_request_timeout),
        }))
    }

    pub fn device_connect_url(&self) -> String {
        let separator = if self.relay_url.contains('?') {
            '&'
        } else {
            '?'
        };

        format!(
            "{}{separator}device_id={}&token={}",
            self.relay_url,
            urlencoding::encode(&self.device_id),
            urlencoding::encode(&self.token)
        )
    }
}

fn relay_enabled() -> bool {
    matches!(
        env::var("PCLOUD_RELAY_ENABLED")
            .unwrap_or_else(|_| "false".to_owned())
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn required_env(name: &str) -> anyhow::Result<String> {
    env::var(name).map_err(|_| anyhow::anyhow!("{name} must be set when relay is enabled"))
}
