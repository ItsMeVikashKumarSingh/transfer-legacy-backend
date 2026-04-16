use chrono::{DateTime, Utc};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug)]
pub struct InviteRow {
    pub invite_id: Uuid,
    pub policy_id: Uuid,
    pub email: String,
    pub role: String,
    pub claim_token_hmac: Vec<u8>,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

pub async fn insert_invite(
    pool: &PgPool,
    invite_id: Uuid,
    policy_id: Uuid,
    email: &str,
    role: &str,
    claim_token_hmac: Vec<u8>,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO notify.invites (invite_id, policy_id, email, role, claim_token_hmac, expires_at) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(invite_id)
    .bind(policy_id)
    .bind(email)
    .bind(role)
    .bind(claim_token_hmac)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_invite_tx(
    tx: &mut Transaction<'_, Postgres>,
    invite_id: Uuid,
    policy_id: Uuid,
    email: &str,
    role: &str,
    claim_token_hmac: Vec<u8>,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO notify.invites (invite_id, policy_id, email, role, claim_token_hmac, expires_at) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(invite_id)
    .bind(policy_id)
    .bind(email)
    .bind(role)
    .bind(claim_token_hmac)
    .bind(expires_at)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

pub async fn fetch_invite_for_update(
    tx: &mut Transaction<'_, Postgres>,
    invite_id: Uuid,
) -> Result<InviteRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, Vec<u8>, DateTime<Utc>, bool)>(
        "SELECT invite_id, policy_id, email, role, claim_token_hmac, expires_at, used FROM notify.invites WHERE invite_id = $1 FOR UPDATE",
    )
    .bind(invite_id)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(InviteRow {
        invite_id: row.0,
        policy_id: row.1,
        email: row.2,
        role: row.3,
        claim_token_hmac: row.4,
        expires_at: row.5,
        used: row.6,
    })
}

pub async fn mark_invite_used(
    tx: &mut Transaction<'_, Postgres>,
    invite_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE notify.invites SET used = true, used_at = now() WHERE invite_id = $1")
        .bind(invite_id)
        .execute(tx.as_mut())
        .await?;
    Ok(())
}
