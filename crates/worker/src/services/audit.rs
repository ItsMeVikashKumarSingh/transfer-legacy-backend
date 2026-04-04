use serde_json::Value;
use base64::Engine as _;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize};

#[derive(thiserror::Error, Debug)]
pub enum AuditError {
    #[error("serialization error")]
    Serialization,
    #[error("db error")]
    Database,
}

pub async fn append_event(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    event_type: &str,
    payload: &Value,
) -> Result<Vec<u8>, AuditError> {
    let payload_bytes = canonicalize(payload).map_err(|_| AuditError::Serialization)?;
    let payload_hash = sha256(&payload_bytes);

    let prev_hash = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT audit_head_hash FROM inheritance.policies WHERE policy_id = $1",
    )
    .bind(policy_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| AuditError::Database)?;

    let event_id = Uuid::new_v4();
    let event_hash_payload = serde_json::json!({
        "event_id": event_id,
        "payload_hash": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&payload_hash),
        "prev_hash": prev_hash.as_ref().map(|h| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(h)),
    });
    let event_hash_bytes = canonicalize(&event_hash_payload).map_err(|_| AuditError::Serialization)?;
    let event_hash = sha256(&event_hash_bytes);

    sqlx::query(
        "INSERT INTO audit.events (event_id, policy_id, event_type, payload_hash, prev_hash, event_hash) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(event_id)
    .bind(policy_id)
    .bind(event_type)
    .bind(payload_hash)
    .bind(prev_hash)
    .bind(event_hash.clone())
    .execute(&mut *tx)
    .await
    .map_err(|_| AuditError::Database)?;

    sqlx::query("UPDATE inheritance.policies SET audit_head_hash = $1 WHERE policy_id = $2")
        .bind(event_hash.clone())
        .bind(policy_id)
        .execute(&mut *tx)
        .await
        .map_err(|_| AuditError::Database)?;

    Ok(event_hash)
}
