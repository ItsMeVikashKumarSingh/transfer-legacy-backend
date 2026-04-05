use sqlx::PgPool;
use uuid::Uuid;

pub async fn fetch_audit_event_hashes(
    pool: &PgPool,
    policy_id: Uuid,
) -> Result<Vec<Vec<u8>>, sqlx::Error> {
    let rows = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT event_hash FROM audit.events WHERE policy_id = $1 ORDER BY created_at ASC",
    )
    .bind(policy_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}

pub async fn fetch_release_record(
    pool: &PgPool,
    policy_id: Uuid,
    claim_id: Uuid,
) -> Result<Option<(Uuid, Vec<u8>, String, i32, String)>, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Vec<u8>, String, i32, String)>(
        "SELECT release_id, payload_hash, signature, schema_version, crypto_version FROM inheritance.release_records WHERE policy_id = $1 AND claim_id = $2 ORDER BY created_at DESC LIMIT 1",
    )
    .bind(policy_id)
    .bind(claim_id)
    .fetch_optional(pool)
    .await?;
    Ok(row)
}

pub async fn fetch_claim_attachments(
    pool: &PgPool,
    claim_id: Uuid,
) -> Result<Vec<(Uuid, String, Option<Vec<u8>>, Option<i64>, Option<String>)>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, String, Option<Vec<u8>>, Option<i64>, Option<String>)>(
        "SELECT attachment_id, object_key, sha256, size_bytes, mime_type FROM inheritance.claim_attachments WHERE claim_id = $1 AND status = 'confirmed'",
    )
    .bind(claim_id)
    .fetch_all(pool)
    .await?;
    Ok(rows)
}
