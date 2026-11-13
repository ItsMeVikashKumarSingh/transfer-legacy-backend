use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CryptoVersion {
    #[serde(rename = "aead-xchacha20p1305-v1")]
    AeadXChaCha20Poly1305V1,
    #[serde(rename = "argon2id-v1")]
    Argon2idV1,
    #[serde(rename = "opaque-v1")]
    OpaqueV1,
    #[serde(rename = "hybrid-wrap-v1")]
    HybridWrapV1,
    #[serde(rename = "ed25519-v1")]
    Ed25519V1,
    #[serde(rename = "dilithium2-v1")]
    Dilithium2V1,
}

impl CryptoVersion {
    pub fn as_str(&self) -> &'static str {
        match self {
            CryptoVersion::AeadXChaCha20Poly1305V1 => "aead-xchacha20p1305-v1",
            CryptoVersion::Argon2idV1 => "argon2id-v1",
            CryptoVersion::OpaqueV1 => "opaque-v1",
            CryptoVersion::HybridWrapV1 => "hybrid-wrap-v1",
            CryptoVersion::Ed25519V1 => "ed25519-v1",
            CryptoVersion::Dilithium2V1 => "dilithium2-v1",
        }
    }

    pub fn is_supported(version: &str) -> bool {
        matches!(
            version,
            "aead-xchacha20p1305-v1"
                | "argon2id-v1"
                | "opaque-v1"
                | "hybrid-wrap-v1"
                | "ed25519-v1"
                | "dilithium2-v1"
        )
    }
}
