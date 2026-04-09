use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug)]
pub struct AuditEventRow {
    pub event_id: Uuid,
    pub event_type: String,
    pub payload_hash: Vec<u8>,
    pub prev_hash: Option<Vec<u8>>,
    pub event_hash: Vec<u8>,
    pub created_at: DateTime<Utc>,
}

pub async fn insert_audit_event(
    tx: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    policy_id: Uuid,
    event_type: &str,
    payload_hash: Vec<u8>,
    prev_hash: Option<Vec<u8>>,
    event_hash: Vec<u8>,
    actor_id: Option<Uuid>,
    ip_hash: Option<Vec<u8>>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit.events (event_id, policy_id, event_type, payload_hash, prev_hash, event_hash, actor_id, ip_hash) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(event_id)
    .bind(policy_id)
    .bind(event_type)
    .bind(payload_hash)
    .bind(prev_hash)
    .bind(event_hash)
    .bind(actor_id)
    .bind(ip_hash)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn fetch_policy_audit_head(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
) -> Result<Option<Vec<u8>>, sqlx::Error> {
    let row = sqlx::query_scalar::<_, Option<Vec<u8>>>(
        "SELECT audit_head_hash FROM inheritance.policies WHERE policy_id = $1",
    )
    .bind(policy_id)
    .fetch_one(tx.as_mut())
    .await?;
    Ok(row)
}

pub async fn update_policy_audit_head(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    audit_head_hash: Vec<u8>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE inheritance.policies SET audit_head_hash = $1 WHERE policy_id = $2",
    )
    .bind(audit_head_hash)
    .bind(policy_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

pub async fn fetch_audit_chain(
    pool: &PgPool,
    policy_id: Uuid,
) -> Result<Vec<AuditEventRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, String, Vec<u8>, Option<Vec<u8>>, Vec<u8>, DateTime<Utc>)>(
        "SELECT event_id, event_type, payload_hash, prev_hash, event_hash, created_at FROM audit.events WHERE policy_id = $1 ORDER BY created_at ASC",
    )
    .bind(policy_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| AuditEventRow {
            event_id: r.0,
            event_type: r.1,
            payload_hash: r.2,
            prev_hash: r.3,
            event_hash: r.4,
            created_at: r.5,
        })
        .collect())
}
