use base64::{engine::general_purpose, Engine};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand_core::OsRng;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct DeviceIdentity {
    pub private_key: String, // base64(32 bytes)
    pub public_key: String,  // base64(32 bytes)
}

impl DeviceIdentity {
    pub fn generate() -> Self {
        let mut rng = OsRng;
        let signing_key = SigningKey::generate(&mut rng);
        let verifying_key = signing_key.verifying_key();

        Self {
            private_key: general_purpose::STANDARD.encode(signing_key.to_bytes()),
            public_key: general_purpose::STANDARD.encode(verifying_key.to_bytes()),
        }
    }

    pub fn device_id(&self) -> String {
        // For commit #1: simplest stable ID
        // Later: change to hash(public_key) for nicer fixed-length IDs
        self.public_key.clone()
    }

    pub fn signing_key(&self) -> SigningKey {
        let bytes: [u8; 32] = general_purpose::STANDARD
            .decode(&self.private_key)
            .expect("invalid base64 private_key")
            .try_into()
            .expect("private_key must be 32 bytes");

        SigningKey::from_bytes(&bytes)
    }

    pub fn verifying_key(&self) -> VerifyingKey {
        let bytes: [u8; 32] = general_purpose::STANDARD
            .decode(&self.public_key)
            .expect("invalid base64 public_key")
            .try_into()
            .expect("public_key must be 32 bytes");

        VerifyingKey::from_bytes(&bytes).expect("invalid public key bytes")
    }
}