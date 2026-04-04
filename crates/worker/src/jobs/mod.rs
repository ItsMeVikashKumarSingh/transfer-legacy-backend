use apalis::prelude::*;
use base64::Engine as _;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::{audit, b2, brevo, notify_log, openbao};
use crate::state::AppState;
use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvalJob {
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditAnchorJob {
    pub date: NaiveDate,
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseEvalJob {
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotifyJob {
    pub policy_id: Option<Uuid>,
    pub email: String,
    pub template_id: String,
    pub params: serde_json::Value,
    pub dedupe_key: String,
    pub attempts: u32,
}

#[derive(thiserror::Error, Debug)]
pub enum JobError {
    #[error("db error")]
    Database,
    #[error("audit error")]
    Audit,
    #[error("notify error")]
    Notify,
    #[error("storage error")]
    Storage,
    #[error("anchor error")]
    Anchor,
    #[error("release error")]
    Release,
}

pub async fn run_heartbeat_eval(
    _: HeartbeatEvalJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let now = Utc::now();
    let pool = &state.db;

    enqueue_owner_reminders(pool, &state, now).await?;

    let mut tx = pool.begin().await.map_err(|_| JobError::Database)?;
    let pending_rows = sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>, DateTime<Utc>, String)>(
        "UPDATE inheritance.policies SET status = 'pending', updated_at = now() WHERE status = 'active' AND pending_at <= now() AND is_deleted = false RETURNING policy_id, owner_id, pending_at, grace_deadline, cadence::text",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| JobError::Database)?;

    for row in pending_rows {
        let payload = serde_json::json!({
            "policy_id": row.0,
            "status": "pending",
            "pending_at": row.2,
            "grace_deadline": row.3,
        });
        audit::append_event(&mut tx, row.0, "policy_pending", &payload)
            .await
            .map_err(|_| JobError::Audit)?;

        if let Some(email) = fetch_user_email(pool, row.1).await {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": row.0,
                "pending_at": row.2,
                "grace_deadline": row.3,
            });
            let dedupe_key = format!("owner-pending-{}-{}", row.0, now.date_naive());
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(row.0),
                    email,
                    template_id: state.config.brevo_owner_reminder_daily_template_id.clone(),
                    params,
                    dedupe_key,
                    attempts: 0,
                },
            )
            .await?;
        }
    }

    let investigating_rows = sqlx::query_as::<_, (Uuid, serde_json::Value, serde_json::Value)>(
        "UPDATE inheritance.policies SET status = 'investigating', updated_at = now() WHERE status = 'pending' AND grace_deadline <= now() AND is_deleted = false RETURNING policy_id, beneficiaries, approvers",
    )
    .fetch_all(&mut *tx)
    .await
    .map_err(|_| JobError::Database)?;

    for row in investigating_rows {
        let payload = serde_json::json!({
            "policy_id": row.0,
            "status": "investigating",
        });
        audit::append_event(&mut tx, row.0, "policy_investigating", &payload)
            .await
            .map_err(|_| JobError::Audit)?;

        for email in extract_emails(&row.1) {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": row.0,
            });
            let dedupe_key = format!("beneficiary-investigating-{}-{}", row.0, email);
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(row.0),
                    email,
                    template_id: state.config.brevo_beneficiary_claim_template_id.clone(),
                    params,
                    dedupe_key,
                    attempts: 0,
                },
            )
            .await?;
        }

        for email in extract_emails(&row.2) {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": row.0,
            });
            let dedupe_key = format!("approver-investigating-{}-{}", row.0, email);
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(row.0),
                    email,
                    template_id: state.config.brevo_approver_attestation_template_id.clone(),
                    params,
                    dedupe_key,
                    attempts: 0,
                },
            )
            .await?;
        }
    }

    tx.commit().await.map_err(|_| JobError::Database)?;
    Ok(())
}

pub async fn run_notify(
    job: NotifyJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let queued = notify_log::insert_queued(
        &state.db,
        job.policy_id,
        &job.email,
        &job.template_id,
        &job.dedupe_key,
    )
    .await
    .map_err(|_| JobError::Database)?;

    if !queued {
        return Ok(());
    }

    let result = brevo::send_template_email(
        &state.config,
        &job.template_id,
        &job.email,
        job.params,
    )
    .await;

    match result {
        Ok(_) => notify_log::mark_sent(&state.db, &job.dedupe_key)
            .await
            .map_err(|_| JobError::Database)?,
        Err(err) => {
            let msg = format!("{:?}", err);
            notify_log::mark_failed(&state.db, &job.dedupe_key, &msg)
                .await
                .map_err(|_| JobError::Database)?;
            return Err(JobError::Notify);
        }
    }

    Ok(())
}

pub async fn run_audit_anchor(
    job: AuditAnchorJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let entries: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT event_hash FROM audit.events WHERE created_at::date = $1 ORDER BY created_at ASC",
    )
    .bind(job.date)
    .fetch_all(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    let encoded: Vec<String> = entries
        .iter()
        .map(|v| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v))
        .collect();

    let snapshot = serde_json::json!({
        "date": job.date.to_string(),
        "event_hashes": encoded,
        "count_audit_entries": entries.len(),
        "server_id": state.config.server_id.clone(),
    });

    let snapshot_bytes = canonicalize(&snapshot).map_err(|_| JobError::Anchor)?;
    let head_hash = sha256(&snapshot_bytes);
    let signature = openbao::sign_digest(&state.config, "tl-signing", &head_hash)
        .await
        .map_err(|_| JobError::Anchor)?;

    let anchor = serde_json::json!({
        "date": job.date.to_string(),
        "head_hash": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&head_hash),
        "count_audit_entries": entries.len(),
        "server_id": state.config.server_id.clone(),
        "signature": signature,
    });

    let anchor_bytes = canonicalize(&anchor).map_err(|_| JobError::Anchor)?;
    let key = format!("{}/anchor.json", job.date);
    b2::upload_anchor(&state.config, &key, anchor_bytes.clone())
        .await
        .map_err(|_| JobError::Anchor)?;

    sqlx::query(
        "INSERT INTO audit.anchors (anchor_date, head_hash, entry_count, snapshot, signature) VALUES ($1,$2,$3,$4,$5) ON CONFLICT (anchor_date) DO NOTHING",
    )
    .bind(job.date)
    .bind(head_hash)
    .bind(entries.len() as i32)
    .bind(snapshot)
    .bind(signature)
    .execute(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    Ok(())
}

pub async fn run_release_eval(
    _: ReleaseEvalJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let rows = sqlx::query_as::<_, (Uuid, serde_json::Value, Uuid)>(
        "SELECT p.policy_id, p.m_of_n, c.claim_id FROM inheritance.policies p JOIN inheritance.claims c ON c.policy_id = p.policy_id WHERE p.policy_type = 'm_of_n' AND p.status = 'investigating' AND c.status = 'confirmed'",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    for row in rows {
        let m = row.1.get("m").and_then(|v| v.as_i64()).unwrap_or(0);
        if m <= 0 {
            continue;
        }
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM inheritance.attestations WHERE claim_id = $1",
        )
        .bind(row.2)
        .fetch_one(&state.db)
        .await
        .map_err(|_| JobError::Database)?;

        if count >= m {
            let mut tx = state.db.begin().await.map_err(|_| JobError::Database)?;
            sqlx::query("UPDATE inheritance.policies SET status = 'release_ready', updated_at = now() WHERE policy_id = $1 AND status = 'investigating'")
                .bind(row.0)
                .execute(&mut *tx)
                .await
                .map_err(|_| JobError::Database)?;

            let payload = serde_json::json!({
                "policy_id": row.0,
                "claim_id": row.2,
                "attestation_count": count,
                "required": m,
            });
            audit::append_event(&mut tx, row.0, "m_of_n_release_ready", &payload)
                .await
                .map_err(|_| JobError::Audit)?;
            tx.commit().await.map_err(|_| JobError::Database)?;
        }
    }

    Ok(())
}

async fn enqueue_owner_reminders(
    pool: &PgPool,
    state: &AppState,
    now: DateTime<Utc>,
) -> Result<(), JobError> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, DateTime<Utc>)>(
        "SELECT policy_id, owner_id, cadence::text, pending_at FROM inheritance.policies WHERE status = 'active' AND pending_at IS NOT NULL AND is_deleted = false",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| JobError::Database)?;

    for row in rows {
        let cadence_days = cadence_days(&row.2);
        let early_offset = Duration::days((cadence_days as f64 * 0.2).ceil() as i64);
        let urgent_offset = Duration::days((cadence_days as f64 * 0.1).ceil() as i64);
        let daily_window = Duration::days(7.min(cadence_days as i64));

        if now >= row.3 - early_offset {
            enqueue_owner_notification(
                pool,
                state,
                row.0,
                row.1,
                &state.config.brevo_owner_reminder_early_template_id,
                "owner-early",
                now,
                row.3,
            )
            .await?;
        }

        if now >= row.3 - urgent_offset {
            enqueue_owner_notification(
                pool,
                state,
                row.0,
                row.1,
                &state.config.brevo_owner_reminder_urgent_template_id,
                "owner-urgent",
                now,
                row.3,
            )
            .await?;
        }

        if now >= row.3 - daily_window && now < row.3 {
            enqueue_owner_notification(
                pool,
                state,
                row.0,
                row.1,
                &state.config.brevo_owner_reminder_daily_template_id,
                "owner-daily",
                now,
                row.3,
            )
            .await?;
        }
    }

    Ok(())
}

async fn enqueue_owner_notification(
    pool: &PgPool,
    state: &AppState,
    policy_id: Uuid,
    owner_id: Uuid,
    template_id: &str,
    dedupe_prefix: &str,
    now: DateTime<Utc>,
    pending_at: DateTime<Utc>,
) -> Result<(), JobError> {
    if let Some(email) = fetch_user_email(pool, owner_id).await {
        let params = serde_json::json!({
            "brand_name": state.config.brand_name.clone(),
            "app_url": state.config.app_url.clone(),
            "policy_id": policy_id,
            "pending_at": pending_at,
        });
        let dedupe_key = format!("{}-{}-{}", dedupe_prefix, policy_id, now.date_naive());
        enqueue_notify(
            state,
            NotifyJob {
                policy_id: Some(policy_id),
                email,
                template_id: template_id.to_string(),
                params,
                dedupe_key,
                attempts: 0,
            },
        )
        .await?;
    }
    Ok(())
}

async fn enqueue_notify(state: &AppState, job: NotifyJob) -> Result<(), JobError> {
    state
        .notify_storage
        .push(job)
        .await
        .map_err(|_| JobError::Storage)?;
    Ok(())
}

async fn fetch_user_email(pool: &PgPool, user_id: Uuid) -> Option<String> {
    sqlx::query_scalar::<_, String>("SELECT email FROM auth.users WHERE id = $1")
        .bind(user_id)
        .fetch_one(pool)
        .await
        .ok()
}

fn extract_emails(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("email").and_then(|e| e.as_str()).map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default()
}

fn cadence_days(cadence: &str) -> i64 {
    match cadence {
        "1w" => 7,
        "15d" => 15,
        "1m" => 30,
        "3m" => 90,
        _ => 30,
    }
}
