use crate::config::Config;
use async_trait::async_trait;
use axum::body::Body;
use axum::extract::{FromRequest, Request};
use axum::http::HeaderMap;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use metrics::counter;
use redis::AsyncCommands;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::errors::ApiError;
use crate::state::AppState;
use transfer_legacy_crypto_core::aead::{decrypt, encrypt, AeadEnvelope as CoreAeadEnvelope};
use transfer_legacy_shared_types::AppError;

#[derive(Debug, Deserialize)]
pub struct AeadEnvelope {
    pub nonce: String,
    pub ciphertext: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AeadResponse {
    pub nonce: String,
    pub ciphertext: String,
}

pub struct AeadJson<T>(pub T);

impl<T> AeadJson<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

#[async_trait]
impl<T> FromRequest<AppState, Body> for AeadJson<T>
where
    T: DeserializeOwned + Send,
{
    type Rejection = ApiError;

    async fn from_request(req: Request<Body>, state: &AppState) -> Result<Self, Self::Rejection> {
        let (parts, body) = req.into_parts();
        let headers = parts.headers;
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| ApiError::app(AppError::BadRequest))?;

        let env: AeadEnvelope =
            serde_json::from_slice(&bytes).map_err(|_| ApiError::app(AppError::BadRequest))?;

        let (seq, device_id, ts) = extract_replay_headers(&headers)?;
        enforce_replay(state, &device_id, seq, ts).await?;

        let config = state.config().await;
        let key = decode_key(&config.server_aead_key_b64)?;
        let nonce = URL_SAFE_NO_PAD
            .decode(env.nonce)
            .map_err(|_| ApiError::app(AppError::AeadIntegrity))?;
        let ciphertext = URL_SAFE_NO_PAD
            .decode(env.ciphertext)
            .map_err(|_| ApiError::app(AppError::AeadIntegrity))?;
        let aad = aad_from_headers(&headers);

        let plaintext = decrypt(&key, &nonce, &ciphertext, &aad)
            .map_err(|_| ApiError::app(AppError::AeadIntegrity))?;

        let value = serde_json::from_slice::<T>(&plaintext)
            .map_err(|_| ApiError::app(AppError::BadRequest))?;
        Ok(AeadJson(value))
    }
}

pub fn wrap_response<T: Serialize>(
    config: &Config,
    headers: &HeaderMap,
    value: &T,
) -> Result<AeadResponse, ApiError> {
    let key = decode_key(&config.server_aead_key_b64)?;
    let plaintext = serde_json::to_vec(value).map_err(|_| ApiError::app(AppError::Internal))?;
    let aad = aad_from_headers(headers);
    let CoreAeadEnvelope { nonce, ciphertext } =
        encrypt(&key, &plaintext, &aad).map_err(|_| ApiError::app(AppError::Internal))?;

    Ok(AeadResponse {
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    })
}

fn decode_key(key_b64: &str) -> Result<Vec<u8>, ApiError> {
    URL_SAFE_NO_PAD
        .decode(key_b64.trim())
        .map_err(|_| ApiError::app(AppError::Internal))
}

fn aad_from_headers(headers: &HeaderMap) -> Vec<u8> {
    let req_id = headers
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let seq = headers
        .get("x-seq")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let ts = headers
        .get("x-timestamp")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    format!("{req_id}|{seq}|{ts}").into_bytes()
}

fn extract_replay_headers(headers: &HeaderMap) -> Result<(u64, String, i64), ApiError> {
    let seq = headers
        .get("x-seq")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<u64>().ok())
        .ok_or_else(|| ApiError::app(AppError::ReplayOrSkew))?;
    let ts = headers
        .get("x-timestamp")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .ok_or_else(|| ApiError::app(AppError::ReplayOrSkew))?;
    let device_id = headers
        .get("x-device-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::app(AppError::ReplayOrSkew))?;

    Ok((seq, device_id, ts))
}

async fn enforce_replay(
    state: &AppState,
    device_id: &str,
    seq: u64,
    ts: i64,
) -> Result<(), ApiError> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| ApiError::app(AppError::ReplayOrSkew))?;
    let now_ts = now.as_secs() as i64;
    if (now_ts - ts).abs() > 300 {
        counter!("aead_failures_total", "reason" => "clock_skew").increment(1);
        return Err(ApiError::app(AppError::ReplayOrSkew));
    }

    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;
    let key = format!("seq:{}", device_id);
    let last: Option<u64> = conn
        .get(&key)
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;

    if let Some(last_seq) = last {
        if seq <= last_seq {
            counter!("nonce_reuse_detected_total").increment(1);
            counter!("aead_failures_total", "reason" => "replay").increment(1);
            return Err(ApiError::app(AppError::ReplayDetected));
        }
    }

    let _: () = conn
        .set_ex(key, seq, 86400)
        .await
        .map_err(|_| ApiError::app(AppError::Internal))?;

    Ok(())
}
