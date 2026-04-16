use chrono::{DateTime, Utc};
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

#[derive(Debug)]
pub struct PolicyRow {
    pub policy_id: Uuid,
    pub owner_id: Uuid,
    pub policy_type: String,
    pub cadence: String,
    pub m_of_n: Option<Value>,
    pub beneficiaries: Value,
    pub approvers: Value,
    pub release_conditions: Option<Value>,
    pub status: String,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub pending_at: Option<DateTime<Utc>>,
    pub grace_deadline: Option<DateTime<Utc>>,
    pub conflict_hold_until: Option<DateTime<Utc>>,
    pub audit_head_hash: Option<Vec<u8>>,
    pub label: Option<String>,
    pub enc_owner_name: Option<Vec<u8>>,
}

pub async fn insert_policy_tx(
    tx: &mut Transaction<'_, Postgres>,
    owner_id: Uuid,
    policy_type: &str,
    cadence: &str,
    m_of_n: Option<Value>,
    beneficiaries: Value,
    approvers: Value,
    release_conditions: Option<Value>,
    status: &str,
    last_heartbeat_at: Option<DateTime<Utc>>,
    pending_at: Option<DateTime<Utc>>,
    grace_deadline: Option<DateTime<Utc>>,
    crypto_version: String,
    schema_version: i32,
    label: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let policy_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO inheritance.policies (policy_id, owner_id, policy_type, cadence, m_of_n, beneficiaries, approvers, release_conditions, status, last_heartbeat_at, pending_at, grace_deadline, crypto_version, schema_version, label) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15)",
    )
    .bind(policy_id)
    .bind(owner_id)
    .bind(policy_type)
    .bind(cadence)
    .bind(m_of_n)
    .bind(beneficiaries)
    .bind(approvers)
    .bind(release_conditions)
    .bind(status)
    .bind(last_heartbeat_at)
    .bind(pending_at)
    .bind(grace_deadline)
    .bind(crypto_version)
    .bind(schema_version)
    .bind(label)
    .execute(tx.as_mut())
    .await?;

    Ok(policy_id)
}

pub async fn update_policy_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    owner_id: Uuid,
    policy_type: &str,
    cadence: &str,
    m_of_n: Option<Value>,
    beneficiaries: Value,
    approvers: Value,
    release_conditions: Option<Value>,
    pending_at: Option<DateTime<Utc>>,
    grace_deadline: Option<DateTime<Utc>>,
    label: Option<&str>,
) -> Result<(), sqlx::Error> {
    let res = sqlx::query(
        "UPDATE inheritance.policies SET policy_type = $1, cadence = $2, m_of_n = $3, beneficiaries = $4, approvers = $5, release_conditions = $6, pending_at = $7, grace_deadline = $8, label = $9, updated_at = now() WHERE policy_id = $10 AND owner_id = $11",
    )
    .bind(policy_type)
    .bind(cadence)
    .bind(m_of_n)
    .bind(beneficiaries)
    .bind(approvers)
    .bind(release_conditions)
    .bind(pending_at)
    .bind(grace_deadline)
    .bind(label)
    .bind(policy_id)
    .bind(owner_id)
    .execute(tx.as_mut())
    .await?;

    if res.rows_affected() == 0 {
        return Err(sqlx::Error::RowNotFound);
    }

    Ok(())
}

pub async fn fetch_policy(pool: &PgPool, policy_id: Uuid) -> Result<PolicyRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, Option<Value>, Value, Value, Option<Value>, String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<Vec<u8>>, Option<String>, Option<Vec<u8>>)>(
        r#"SELECT 
            p.policy_id, p.owner_id, p.policy_type::text, p.cadence::text, p.m_of_n, p.beneficiaries, p.approvers, p.release_conditions, p.status::text, p.last_heartbeat_at, p.pending_at, p.grace_deadline, p.conflict_hold_until, p.audit_head_hash, p.label, per.enc_legal_name
        FROM inheritance.policies p
        LEFT JOIN auth_ext.person_user_links pul ON pul.user_id = p.owner_id
        LEFT JOIN auth_ext.persons per ON per.person_id = pul.person_id
        WHERE p.policy_id = $1 AND p.is_deleted = false"#,
    )
    .bind(policy_id)
    .fetch_one(pool)
    .await?;

    Ok(PolicyRow {
        policy_id: row.0,
        owner_id: row.1,
        policy_type: row.2,
        cadence: row.3,
        m_of_n: row.4,
        beneficiaries: row.5,
        approvers: row.6,
        release_conditions: row.7,
        status: row.8,
        last_heartbeat_at: row.9,
        pending_at: row.10,
        grace_deadline: row.11,
        conflict_hold_until: row.12,
        audit_head_hash: row.13,
        label: row.14,
        enc_owner_name: row.15,
    })
}

pub async fn fetch_policy_for_update_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
) -> Result<PolicyRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, Option<Value>, Value, Value, Option<Value>, String, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<DateTime<Utc>>, Option<Vec<u8>>, Option<String>, Option<Vec<u8>>)>(
        r#"SELECT 
            p.policy_id, p.owner_id, p.policy_type::text, p.cadence::text, p.m_of_n, p.beneficiaries, p.approvers, p.release_conditions, p.status::text, p.last_heartbeat_at, p.pending_at, p.grace_deadline, p.conflict_hold_until, p.audit_head_hash, p.label, per.enc_legal_name
        FROM inheritance.policies p
        LEFT JOIN auth_ext.person_user_links pul ON pul.user_id = p.owner_id
        LEFT JOIN auth_ext.persons per ON per.person_id = pul.person_id
        WHERE p.policy_id = $1 AND p.is_deleted = false FOR UPDATE"#,
    )
    .bind(policy_id)
    .fetch_one(tx.as_mut())
    .await?;

    Ok(PolicyRow {
        policy_id: row.0,
        owner_id: row.1,
        policy_type: row.2,
        cadence: row.3,
        m_of_n: row.4,
        beneficiaries: row.5,
        approvers: row.6,
        release_conditions: row.7,
        status: row.8,
        last_heartbeat_at: row.9,
        pending_at: row.10,
        grace_deadline: row.11,
        conflict_hold_until: row.12,
        audit_head_hash: row.13,
        label: row.14,
        enc_owner_name: row.15,
    })
}

pub async fn update_policy_heartbeat_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    last_heartbeat_at: DateTime<Utc>,
    pending_at: DateTime<Utc>,
    grace_deadline: DateTime<Utc>,
    new_status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE inheritance.policies SET last_heartbeat_at = $1, pending_at = $2, grace_deadline = $3, status = $4, updated_at = now() WHERE policy_id = $5",
    )
    .bind(last_heartbeat_at)
    .bind(pending_at)
    .bind(grace_deadline)
    .bind(new_status)
    .bind(policy_id)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn insert_heartbeat_tx(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    device_id: Uuid,
    device_sig: Vec<u8>,
    ts: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO inheritance.heartbeats (policy_id, device_id, device_sig, ts) VALUES ($1,$2,$3,$4)",
    )
    .bind(policy_id)
    .bind(device_id)
    .bind(device_sig)
    .bind(ts)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}

pub async fn update_policy_participants(
    tx: &mut Transaction<'_, Postgres>,
    policy_id: Uuid,
    beneficiaries: Value,
    approvers: Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE inheritance.policies SET beneficiaries = $1, approvers = $2, updated_at = now() WHERE policy_id = $3",
    )
    .bind(beneficiaries)
    .bind(approvers)
    .bind(policy_id)
    .execute(tx.as_mut())
    .await?;
    Ok(())
}
