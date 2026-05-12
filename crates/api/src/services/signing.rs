use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use ed25519_dalek::{SigningKey, Signer};
use reqwest::Client;
use serde::Deserialize;
use std::sync::Arc;

#[async_trait]
pub trait TransitSigner: Send + Sync {
    async fn sign_digest(&self, key_name: &str, digest: &[u8]) -> Result<String, anyhow::Error>;
}

pub struct OpenBaoSigner {
    openbao_addr: String,
    openbao_token: String,
}

impl OpenBaoSigner {
    pub fn new(openbao_addr: String, openbao_token: String) -> Self {
        Self {
            openbao_addr,
            openbao_token,
        }
    }
}

#[derive(Deserialize)]
struct SignResponseData {
    signature: String,
}

#[derive(Deserialize)]
struct SignResponse {
    data: SignResponseData,
}

#[async_trait]
impl TransitSigner for OpenBaoSigner {
    async fn sign_digest(&self, key_name: &str, digest: &[u8]) -> Result<String, anyhow::Error> {
        let client = Client::new();
        let url = format!("{}/v1/transit/sign/{}", self.openbao_addr, key_name);
        let input_b64 = STANDARD.encode(digest);

        let res = client
            .post(url)
            .header("X-Vault-Token", self.openbao_token.as_str())
            .json(&serde_json::json!({ "input": input_b64 }))
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(anyhow::anyhow!("OpenBao transit signing HTTP error: {}", res.status()));
        }

        let body: SignResponse = res.json().await?;
        Ok(body.data.signature)
    }
}

pub struct InMemorySigner {
    signing_key: SigningKey,
}

impl InMemorySigner {
    pub fn new(private_key_b64: &str) -> Result<Self, anyhow::Error> {
        if private_key_b64.is_empty() {
            tracing::warn!("⚠️ No SERVER_PRIVATE_KEY_B64 provided! Generating ephemeral one-time Ed25519 signing key for development.");
            let key_bytes_32 = rand::random::<[u8; 32]>();
            let signing_key = SigningKey::from_bytes(&key_bytes_32);
            return Ok(Self { signing_key });
        }
        let key_bytes = STANDARD.decode(private_key_b64.trim())?;
        let key_bytes_32: [u8; 32] = key_bytes.as_slice().try_into()
            .map_err(|_| anyhow::anyhow!("Invalid private key length: expected 32 bytes"))?;
        let signing_key = SigningKey::from_bytes(&key_bytes_32);
        Ok(Self { signing_key })
    }
}

#[async_trait]
impl TransitSigner for InMemorySigner {
    async fn sign_digest(&self, _key_name: &str, digest: &[u8]) -> Result<String, anyhow::Error> {
        let signature = self.signing_key.sign(digest);
        let sig_b64 = STANDARD.encode(signature.to_bytes());
        Ok(format!("vault:v1:{}", sig_b64))
    }
}
