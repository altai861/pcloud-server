use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct PairingRequest {

    device_name: String,
    ip: String
}

pub async fn list_pairings() -> Json<Vec<PairingRequest>> {

    Json(vec![
        PairingRequest {
            device_name: "iPhone".into(),
            ip: "192.168.1.50".into()
        }
    ])
}