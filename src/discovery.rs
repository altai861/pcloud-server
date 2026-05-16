use crate::relay::config::RelayClientConfig;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use std::{env, net::SocketAddr, time::Duration};

const SERVICE_TYPE: &str = "_pcloud._tcp.local.";
const DEFAULT_SERVICE_NAME: &str = "PCloud Device";
const API_PATH: &str = "/api/client";

#[derive(Clone, Debug)]
pub struct MdnsDiscoveryConfig {
    pub service_name: String,
    pub host_name: String,
    pub port: u16,
    pub device_id: Option<String>,
    pub relay_base_url: Option<String>,
}

impl MdnsDiscoveryConfig {
    pub fn from_env(
        client_bind: SocketAddr,
        relay_config: Option<&RelayClientConfig>,
    ) -> Option<Self> {
        if !mdns_enabled() {
            return None;
        }

        let device_id = env::var("PCLOUD_DEVICE_ID")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty());

        let service_name = env::var("PCLOUD_MDNS_NAME")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                device_id
                    .as_ref()
                    .map(|device_id| format!("PCloud {device_id}"))
                    .unwrap_or_else(|| DEFAULT_SERVICE_NAME.to_owned())
            });

        let host_label = env::var("PCLOUD_MDNS_HOSTNAME")
            .ok()
            .map(|value| value.trim().to_owned())
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| {
                device_id
                    .as_deref()
                    .map(sanitize_dns_label)
                    .filter(|value| !value.is_empty())
                    .unwrap_or_else(|| format!("pcloud-{}", client_bind.port()))
            });

        let relay_base_url = env::var("PCLOUD_RELAY_PUBLIC_BASE_URL")
            .ok()
            .map(|value| value.trim().trim_end_matches('/').to_owned())
            .filter(|value| !value.is_empty())
            .or_else(|| relay_config.map(|config| derive_public_relay_base_url(&config.relay_url)));

        Some(Self {
            service_name,
            host_name: format!("{host_label}.local."),
            port: client_bind.port(),
            device_id,
            relay_base_url,
        })
    }
}

pub fn run_mdns_discovery(config: MdnsDiscoveryConfig) {
    if let Err(error) = register_mdns_service(config) {
        eprintln!("mDNS discovery disabled: {error}");
    }
}

fn register_mdns_service(config: MdnsDiscoveryConfig) -> anyhow::Result<()> {
    let mdns = ServiceDaemon::new()?;
    let properties = build_txt_properties(&config);
    let service = ServiceInfo::new(
        SERVICE_TYPE,
        &config.service_name,
        &config.host_name,
        "",
        config.port,
        properties.as_slice(),
    )?
    .enable_addr_auto();

    let fullname = service.get_fullname().to_owned();
    mdns.register(service)?;
    println!("mDNS discovery enabled: {fullname} on port {}", config.port);

    loop {
        std::thread::sleep(Duration::from_secs(60 * 60));
    }
}

fn build_txt_properties(config: &MdnsDiscoveryConfig) -> Vec<(String, String)> {
    let mut properties = vec![
        ("version".to_owned(), "1".to_owned()),
        ("api_path".to_owned(), API_PATH.to_owned()),
        ("protocol".to_owned(), "http".to_owned()),
    ];

    if let Some(device_id) = &config.device_id {
        properties.push(("device_id".to_owned(), device_id.clone()));
        properties.push(("relay_path".to_owned(), format!("/d/{device_id}")));
    }

    if let Some(relay_base_url) = &config.relay_base_url {
        properties.push(("relay_base_url".to_owned(), relay_base_url.clone()));
    }

    properties
}

fn mdns_enabled() -> bool {
    !matches!(
        env::var("PCLOUD_MDNS_ENABLED")
            .unwrap_or_else(|_| "true".to_owned())
            .trim()
            .to_ascii_lowercase()
            .as_str(),
        "0" | "false" | "no" | "off"
    )
}

fn derive_public_relay_base_url(relay_url: &str) -> String {
    let base = relay_url
        .split('?')
        .next()
        .unwrap_or(relay_url)
        .trim_end_matches("/api/relay/device/connect")
        .trim_end_matches('/');

    if let Some(rest) = base.strip_prefix("wss://") {
        format!("https://{rest}")
    } else if let Some(rest) = base.strip_prefix("ws://") {
        format!("http://{rest}")
    } else {
        base.to_owned()
    }
}

fn sanitize_dns_label(value: &str) -> String {
    let mut label = String::with_capacity(value.len());

    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            label.push(character.to_ascii_lowercase());
        } else if character == '-' || character == '_' || character.is_whitespace() {
            label.push('-');
        }
    }

    let label = label.trim_matches('-');
    if label.len() > 63 {
        label[..63].trim_matches('-').to_owned()
    } else {
        label.to_owned()
    }
}
