use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug)]
pub struct ClaimRow {
    pub claim_id: Uuid,
    pub policy_id: Uuid,
    pub claimant_person_id: Uuid,
    pub claim_type: String,
    pub status: String,
    pub confirmation_deadline: Option<DateTime<Utc>>,
}

pub async fn insert_claim_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    claimant_person_id: Uuid,
    claim_type: &str,
    status: &str,
    confirmation_deadline: Option<DateTime<Utc>>,
) -> Result<Uuid, sqlx::Error> {
    let claim_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inheritance.claims (claim_id, policy_id, claimant_person_id, claim_type, status, confirmation_deadline) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(claim_id)
    .bind(policy_id)
    .bind(claimant_person_id)
    .bind(claim_type)
    .bind(status)
    .bind(confirmation_deadline)
    .execute(tx.as_mut())
    .await?;
    Ok(claim_id)
}

pub async fn fetch_claim_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    claim_id: Uuid,
) -> Result<ClaimRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, String, Option<DateTime<Utc>>)>(
        "SELECT claim_id, policy_id, claimant_person_id, claim_type::text, status::text, confirmation_deadline FROM inheritance.claims WHERE claim_id = $1 FOR UPDATE",
    )
    .bind(claim_id)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(ClaimRow {
        claim_id: row.0,
        policy_id: row.1,
        claimant_person_id: row.2,
        claim_type: row.3,
        status: row.4,
        confirmation_deadline: row.5,
    })
}

pub async fn update_claim_status_tx(
    tx: &mut Transaction<'_, Postgres>,
    claim_id: Uuid,
    status: &str,
    confirmed_at: Option<DateTime<Utc>>,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE inheritance.claims SET status = $1, confirmed_at = $2 WHERE claim_id = $3")
        .bind(status)
        .bind(confirmed_at)
        .bind(claim_id)
        .execute(tx.as_mut())
        .await?;
    Ok(())
}

pub async fn insert_attachment_tx(
    tx: &mut Transaction<'_, Postgres>,
    attachment_id: Uuid,
    claim_id: Uuid,
    object_key: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO inheritance.claim_attachments (attachment_id, claim_id, object_key) VALUES ($1,$2,$3)",
    )
    .bind(attachment_id)
    .bind(claim_id)
    .bind(object_key)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

pub async fn confirm_attachment_tx(
    tx: &mut Transaction<'_, Postgres>,
    attachment_id: Uuid,
    sha256: Vec<u8>,
    size_bytes: i64,
    mime_type: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE inheritance.claim_attachments SET sha256 = $1, size_bytes = $2, mime_type = $3, status = 'confirmed', confirmed_at = now() WHERE attachment_id = $4",
    )
    .bind(sha256)
    .bind(size_bytes)
    .bind(mime_type)
    .bind(attachment_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

pub async fn fetch_attachment_policy_tx(
    tx: &mut Transaction<'_, Postgres>,
    attachment_id: Uuid,
) -> Result<(Uuid, Uuid), sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT ca.claim_id, c.policy_id FROM inheritance.claim_attachments ca JOIN inheritance.claims c ON c.claim_id = ca.claim_id WHERE ca.attachment_id = $1 FOR UPDATE",
    )
    .bind(attachment_id)
    .fetch_one(tx.as_mut())
    .await?;
    Ok(row)
}

pub async fn insert_attestation_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    claim_id: Uuid,
    approver_person_id: Uuid,
    statement: serde_json::Value,
    signature: Vec<u8>,
    signature_type: &str,
) -> Result<Uuid, sqlx::Error> {
    let attestation_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inheritance.attestations (attestation_id, policy_id, claim_id, approver_person_id, statement, signature, signature_type) VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(attestation_id)
    .bind(policy_id)
    .bind(claim_id)
    .bind(approver_person_id)
    .bind(statement)
    .bind(signature)
    .bind(signature_type)
    .execute(tx.as_mut())
    .await?;
    Ok(attestation_id)
}

pub async fn insert_release_record_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    claim_id: Uuid,
    payload_hash: Vec<u8>,
    signature: &str,
    schema_version: i32,
    crypto_version: &str,
) -> Result<Uuid, sqlx::Error> {
    let release_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inheritance.release_records (release_id, policy_id, claim_id, payload_hash, signature, schema_version, crypto_version) VALUES ($1,$2,$3,$4,$5,$6,$7)",
    )
    .bind(release_id)
    .bind(policy_id)
    .bind(claim_id)
    .bind(payload_hash)
    .bind(signature)
    .bind(schema_version)
    .bind(crypto_version)
    .execute(tx.as_mut())
    .await?;
    Ok(release_id)
}

pub async fn fetch_policy_approvers(
    pool: &PgPool,
    policy_id: Uuid,
) -> Result<serde_json::Value, sqlx::Error> {
    let row = sqlx::query_scalar::<_, serde_json::Value>(
        "SELECT approvers FROM inheritance.policies WHERE policy_id = $1",
    )
    .bind(policy_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn fetch_claim_policy(pool: &PgPool, claim_id: Uuid) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query_scalar::<_, Uuid>(
        "SELECT policy_id FROM inheritance.claims WHERE claim_id = $1",
    )
    .bind(claim_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}
