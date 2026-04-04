use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::Value;
use sqlx::Postgres;
use sqlx::Transaction;
use uuid::Uuid;

use crate::db::queries::audit::{
    fetch_policy_audit_head, insert_audit_event, update_policy_audit_head,
};
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
    actor_id: Option<Uuid>,
    ip_hash: Option<Vec<u8>>,
) -> Result<Vec<u8>, AuditError> {
    let payload_bytes = canonicalize(payload).map_err(|_| AuditError::Serialization)?;
    let payload_hash = sha256(&payload_bytes);

    let prev_hash = fetch_policy_audit_head(tx, policy_id)
        .await
        .map_err(|_| AuditError::Database)?;

    let event_id = Uuid::new_v4();
    let event_hash_payload = serde_json::json!({
        "event_id": event_id,
        "payload_hash": URL_SAFE_NO_PAD.encode(&payload_hash),
        "prev_hash": prev_hash.as_ref().map(|h| URL_SAFE_NO_PAD.encode(h)),
    });
    let event_hash_bytes = canonicalize(&event_hash_payload).map_err(|_| AuditError::Serialization)?;
    let event_hash = sha256(&event_hash_bytes);

    insert_audit_event(
        tx,
        event_id,
        policy_id,
        event_type,
        payload_hash,
        prev_hash,
        event_hash.clone(),
        actor_id,
        ip_hash,
    )
    .await
    .map_err(|_| AuditError::Database)?;

    update_policy_audit_head(tx, policy_id, event_hash.clone())
        .await
        .map_err(|_| AuditError::Database)?;

    Ok(event_hash)
}
