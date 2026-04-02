use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareEnvelope {
    pub ciphertext: String,
    pub crypto_version: String,
}
