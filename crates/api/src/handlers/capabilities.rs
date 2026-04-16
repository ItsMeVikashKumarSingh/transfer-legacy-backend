use axum::extract::Extension;
use axum::Json;
use serde::Serialize;
use tower_http::request_id::RequestId;

use crate::errors::success;
use transfer_legacy_shared_types::{CryptoVersion, CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Serialize)]
pub struct ServerCapabilities {
    pub crypto_versions: Vec<&'static str>,
    pub current_crypto_version: &'static str,
    pub current_schema_version: i32,
    pub aead: &'static str,
    pub kdf: &'static str,
    pub opaque_version: &'static str,
    pub opaque_group: &'static str,
    pub hybrid_kem: &'static str,
    pub signatures: Vec<&'static str>,
    pub canonicalization: &'static str,
}

pub async fn capabilities(
    Extension(request_id): Extension<RequestId>,
) -> Json<crate::errors::SuccessEnvelope<ServerCapabilities>> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let capabilities = ServerCapabilities {
        crypto_versions: vec![CryptoVersion::V1.as_str()],
        current_crypto_version: CURRENT_CRYPTO_VERSION.as_str(),
        current_schema_version: CURRENT_SCHEMA_VERSION,
        aead: "xchacha20-poly1305",
        kdf: "argon2id",
        opaque_version: "opaque-ke v4",
        opaque_group: "ristretto255",
        hybrid_kem: "x25519+kyber768",
        signatures: vec!["ed25519", "dilithium2-optional"],
        canonicalization: "rfc8785",
    };

    success(&rid, capabilities)
}
