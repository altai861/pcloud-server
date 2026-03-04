mod identity;
mod error;

use identity::DeviceIdentity;
use std::fs;
use std::path::Path;

const IDENTITY_FILE: &str = "identity.json";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let identity = if Path::new(IDENTITY_FILE).exists() {
        let data = fs::read_to_string(IDENTITY_FILE)?;
        serde_json::from_str::<DeviceIdentity>(&data)?
    } else {
        let identity = DeviceIdentity::generate();
        let json = serde_json::to_string_pretty(&identity)?;
        fs::write(IDENTITY_FILE, json)?;
        identity
    };

    println!("Device ID: {}", identity.device_id());

    Ok(())
}