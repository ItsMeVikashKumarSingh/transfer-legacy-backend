use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::Client;
use serde::Deserialize;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum OpenBaoError {
    #[error("http error")]
    Http,
    #[error("unexpected response")]
    Unexpected,
}

#[derive(Deserialize)]
struct SignResponseData {
    signature: String,
}

#[derive(Deserialize)]
struct SignResponse {
    data: SignResponseData,
}

pub async fn sign_digest(
    config: &Config,
    key_name: &str,
    digest: &[u8],
) -> Result<String, OpenBaoError> {
    let client = Client::new();
    let url = format!("{}/v1/transit/sign/{}", config.openbao_addr, key_name);
    let input_b64 = STANDARD.encode(digest);

    let res = client
        .post(url)
        .header("X-Vault-Token", config.openbao_token.as_str())
        .json(&serde_json::json!({ "input": input_b64 }))
        .send()
        .await
        .map_err(|_| OpenBaoError::Http)?;

    if !res.status().is_success() {
        return Err(OpenBaoError::Unexpected);
    }

    let body: SignResponse = res.json().await.map_err(|_| OpenBaoError::Unexpected)?;
    Ok(body.data.signature)
}
