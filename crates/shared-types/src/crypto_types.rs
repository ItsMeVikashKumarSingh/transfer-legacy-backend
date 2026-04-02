use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CryptoVersion {
    V1,
}

impl CryptoVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            CryptoVersion::V1 => "v1",
        }
    }
}

pub const CURRENT_CRYPTO_VERSION: CryptoVersion = CryptoVersion::V1;
