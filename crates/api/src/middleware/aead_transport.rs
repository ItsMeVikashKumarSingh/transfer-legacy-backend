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

        let plaintext = decrypt(&key, &nonce, &ciphertext, &aad).map_err(|_| {
            let err = ApiError::app(AppError::AeadIntegrity);
            tracing::error!("AEAD Decryption failed: {:?}", err);
            err
        })?;

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
    let plaintext = serde_json::to_vec(value).map_err(|e| {
        tracing::error!("wrap_response: Failed to serialize response: {:?}", e);
        ApiError::app(AppError::Internal)
    })?;
    let aad = aad_from_headers(headers);
    let CoreAeadEnvelope { nonce, ciphertext } =
        encrypt(&key, &plaintext, &aad).map_err(|e| {
            tracing::error!("wrap_response: Failed to encrypt response: {:?}", e);
            ApiError::app(AppError::Internal)
        })?;

    Ok(AeadResponse {
        nonce: URL_SAFE_NO_PAD.encode(nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext),
    })
}

fn decode_key(key_b64: &str) -> Result<Vec<u8>, ApiError> {
    URL_SAFE_NO_PAD
        .decode(key_b64.trim())
        .map_err(|e| {
            tracing::error!("decode_key: Base64 decoding failed for key_b64: {:?}", e);
            ApiError::app(AppError::Internal)
        })
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
    let systime_now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| {
            tracing::error!("enforce_replay: System time calculation error: {:?}", e);
            ApiError::app(AppError::ReplayOrSkew)
        })?;
    let now_ts = systime_now.as_secs() as i64;
    if (now_ts - ts).abs() > 300 {
        tracing::error!("enforce_replay: Clock skew detected. Now: {}, received ts: {}, diff: {}", now_ts, ts, (now_ts - ts).abs());
        counter!("aead_failures_total", "reason" => "clock_skew").increment(1);
        return Err(ApiError::app(AppError::ReplayOrSkew));
    }

    let mut conn = state.redis_conn.clone();
    let key = format!("seq:{}", device_id);

    // GET with 1 retry to handle idle connection drops gracefully
    let last: Option<u64> = match conn.get(&key).await {
        Ok(val) => Ok(val),
        Err(e) => {
            tracing::warn!("enforce_replay: Redis GET failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            retry_conn.get(&key).await.map_err(|err| {
                tracing::error!("enforce_replay: Redis GET retry failed for key '{}': {:?}", key, err);
                ApiError::app(AppError::Internal)
            })
        }
    }?;

    if let Some(last_seq) = last {
        if seq <= last_seq {
            let config = state.config().await;
            if config.environment == crate::config::Environment::Local && seq == 1 {
                tracing::warn!("🔄 Local dev: Client sequence reset detected (seq = 1). Resetting stored sequence.");
            } else {
                tracing::error!("enforce_replay: Replay detected. client seq: {}, last stored seq: {}", seq, last_seq);
                counter!("nonce_reuse_detected_total").increment(1);
                counter!("aead_failures_total", "reason" => "replay").increment(1);
                return Err(ApiError::app(AppError::ReplayDetected));
            }
        }
    }

    // SET_EX with 1 retry to handle idle connection drops gracefully
    let set_res: Result<(), redis::RedisError> = conn.set_ex(&key, seq, 86400).await;
    let _: () = match set_res {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!("enforce_replay: Redis SET failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            let retry_res: Result<(), redis::RedisError> = retry_conn.set_ex(&key, seq, 86400).await;
            retry_res.map_err(|err| {
                tracing::error!("enforce_replay: Redis SET retry failed for key '{}': {:?}", key, err);
                ApiError::app(AppError::Internal)
            })
        }
    }?;

    Ok(())
}
