use apalis::prelude::*;
use base64::Engine as _;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use metrics::{counter, gauge, histogram};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::services::{audit, b2, brevo, notify_log, openbao};
use crate::state::AppState;
use transfer_legacy_crypto_core::{
    aead::{decrypt, Key},
    hash::sha256,
    jcs::canonicalize,
};

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
pub struct ConflictCheckJob {
    pub attempts: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseDeliveryJob {
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

impl Job for HeartbeatEvalJob {
    const NAME: &'static str = "heartbeat_eval";
}

impl Job for AuditAnchorJob {
    const NAME: &'static str = "audit_anchor";
}

impl Job for ReleaseEvalJob {
    const NAME: &'static str = "release_eval";
}

impl Job for ConflictCheckJob {
    const NAME: &'static str = "conflict_check";
}

impl Job for ReleaseDeliveryJob {
    const NAME: &'static str = "release_delivery";
}

impl Job for NotifyJob {
    const NAME: &'static str = "notify";
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
    #[error("conflict error")]
    Conflict,
}

pub async fn run_heartbeat_eval(
    _: HeartbeatEvalJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let now = Utc::now();
    let pool = &state.db;
    gauge!("job_queue_depth", "job_type" => "heartbeat_eval").set(0.0);

    enqueue_owner_reminders(pool, &state, now).await?;

    let mut tx = pool.begin().await.map_err(|_| JobError::Database)?;
    let pending_rows = sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>, DateTime<Utc>, String, String, String)>(
        "UPDATE inheritance.policies p SET status = 'pending', updated_at = now() 
         FROM auth_ext.persons pr 
         WHERE p.status = 'active' AND p.pending_at <= now() AND p.is_deleted = false AND pr.user_id = p.owner_id
         RETURNING p.policy_id, p.owner_id, p.pending_at, p.grace_deadline, p.cadence::text, p.label, pr.enc_legal_name",
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(|_| JobError::Database)?;

    for row in pending_rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let pending_at = row.2;
        let grace_deadline = row.3;
        let policy_name = row.5;
        let enc_owner_name = row.6;

        let owner_name = decrypt_owner_name(&state.config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let lag = (now - pending_at).num_seconds().max(0) as f64;
        histogram!("heartbeat_worker_lag_seconds").record(lag);
        let payload = serde_json::json!({
            "policy_id": policy_id,
            "status": "pending",
            "pending_at": pending_at,
            "grace_deadline": grace_deadline,
        });
        audit::append_event(&mut tx, policy_id, "policy_pending", &payload)
            .await
            .map_err(|_| JobError::Audit)?;

        if let Some(email) = fetch_user_email(pool, owner_id).await {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": policy_id,
                "pending_at": pending_at,
                "grace_deadline": grace_deadline,
                "owner_name": owner_name,
                "policy_name": policy_name,
            });
            let dedupe_key = format!("owner-pending-{}-{}", policy_id, now.date_naive());
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(policy_id),
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

    let investigating_rows = sqlx::query_as::<_, (Uuid, Uuid, serde_json::Value, serde_json::Value, String, String)>(
        "UPDATE inheritance.policies p SET status = 'investigating', updated_at = now() 
         FROM auth_ext.persons pr 
         WHERE p.status = 'pending' AND p.grace_deadline <= now() AND p.is_deleted = false AND pr.user_id = p.owner_id
         RETURNING p.policy_id, p.owner_id, p.beneficiaries, p.approvers, p.label, pr.enc_legal_name",
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(|_| JobError::Database)?;

    for row in investigating_rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let beneficiaries = row.2;
        let approvers = row.3;
        let policy_name = row.4;
        let enc_owner_name = row.5;

        let owner_name = decrypt_owner_name(&state.config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let payload = serde_json::json!({
            "policy_id": policy_id,
            "status": "investigating",
        });
        audit::append_event(&mut tx, policy_id, "policy_investigating", &payload)
            .await
            .map_err(|_| JobError::Audit)?;

        for email in extract_emails(&beneficiaries) {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": policy_id,
                "owner_name": owner_name,
                "policy_name": policy_name,
            });
            let dedupe_key = format!("beneficiary-investigating-{}-{}", policy_id, email);
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(policy_id),
                    email,
                    template_id: state.config.brevo_beneficiary_claim_template_id.clone(),
                    params,
                    dedupe_key,
                    attempts: 0,
                },
            )
            .await?;
        }

        for email in extract_emails(&approvers) {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": policy_id,
                "owner_name": owner_name,
                "policy_name": policy_name,
            });
            let dedupe_key = format!("approver-investigating-{}-{}", policy_id, email);
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(policy_id),
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

pub async fn run_notify(job: NotifyJob, state: Data<AppState>) -> Result<(), JobError> {
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

    let result =
        brevo::send_template_email(&state.config, &job.template_id, &job.email, job.params).await;

    match result {
        Ok(_) => notify_log::mark_sent(&state.db, &job.dedupe_key)
            .await
            .map_err(|_| JobError::Database)?,
        Err(err) => {
            counter!("api_errors_total", "route" => "worker_notify", "status" => "500")
                .increment(1);
            let msg = format!("{:?}", err);
            notify_log::mark_failed(&state.db, &job.dedupe_key, &msg)
                .await
                .map_err(|_| JobError::Database)?;
            return Err(JobError::Notify);
        }
    }

    Ok(())
}

pub async fn run_audit_anchor(job: AuditAnchorJob, state: Data<AppState>) -> Result<(), JobError> {
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
    gauge!("job_queue_depth", "job_type" => "audit_anchor").set(0.0);

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

pub async fn run_release_eval(_: ReleaseEvalJob, state: Data<AppState>) -> Result<(), JobError> {
    gauge!("job_queue_depth", "job_type" => "release_eval").set(0.0);
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
            sqlx::query("UPDATE inheritance.policies SET status = 'release_ready', conflict_hold_until = now() + interval '48 hours', updated_at = now() WHERE policy_id = $1 AND status = 'investigating'")
                .bind(row.0)
                .execute(tx.as_mut())
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

pub async fn run_conflict_check(
    _: ConflictCheckJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let conflicts = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT policy_id, COUNT(DISTINCT claimant_person_id) FROM inheritance.claims WHERE status = 'confirmed' GROUP BY policy_id HAVING COUNT(DISTINCT claimant_person_id) > 1",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    for row in conflicts {
        let mut tx = state.db.begin().await.map_err(|_| JobError::Database)?;
        let updated = sqlx::query(
            "UPDATE inheritance.policies SET status = 'conflict_pending', conflict_hold_until = now() + interval '48 hours', updated_at = now() WHERE policy_id = $1 AND status = 'release_ready'",
        )
        .bind(row.0)
        .execute(tx.as_mut())
        .await
        .map_err(|_| JobError::Database)?;

        if updated.rows_affected() > 0 {
            let details = serde_json::json!({
                "distinct_claimants": row.1,
            });
            sqlx::query(
                "INSERT INTO ops.conflict_records (conflict_id, policy_id, reason, details) VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(row.0)
            .bind("multiple_confirmed_claimants")
            .bind(details)
            .execute(tx.as_mut())
            .await
            .map_err(|_| JobError::Database)?;

            let payload = serde_json::json!({
                "policy_id": row.0,
                "reason": "multiple_confirmed_claimants",
            });
            audit::append_event(&mut tx, row.0, "conflict_detected", &payload)
                .await
                .map_err(|_| JobError::Audit)?;
        }

        tx.commit().await.map_err(|_| JobError::Database)?;
    }

    let manual_rows = sqlx::query_as::<_, (Uuid,)>(
        "SELECT policy_id FROM inheritance.policies WHERE status = 'conflict_pending' AND conflict_hold_until <= now()",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    for row in manual_rows {
        let mut tx = state.db.begin().await.map_err(|_| JobError::Database)?;
        sqlx::query(
            "UPDATE inheritance.policies SET status = 'manual_review', updated_at = now() WHERE policy_id = $1 AND status = 'conflict_pending'",
        )
        .bind(row.0)
        .execute(tx.as_mut())
        .await
        .map_err(|_| JobError::Database)?;

        sqlx::query("INSERT INTO ops.manual_reviews (review_id, policy_id) VALUES ($1,$2)")
            .bind(Uuid::new_v4())
            .bind(row.0)
            .execute(tx.as_mut())
            .await
            .map_err(|_| JobError::Database)?;

        let payload = serde_json::json!({
            "policy_id": row.0,
            "status": "manual_review",
        });
        audit::append_event(&mut tx, row.0, "manual_review_opened", &payload)
            .await
            .map_err(|_| JobError::Audit)?;
        tx.commit().await.map_err(|_| JobError::Database)?;
    }

    Ok(())
}

pub async fn run_release_delivery(
    _: ReleaseDeliveryJob,
    state: Data<AppState>,
) -> Result<(), JobError> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, serde_json::Value, String, String)>(
        "SELECT p.policy_id, p.owner_id, p.beneficiaries, p.label, pr.enc_legal_name 
         FROM inheritance.policies p 
         JOIN auth_ext.persons pr ON pr.user_id = p.owner_id
         WHERE p.status = 'release_ready' AND p.conflict_hold_until <= now()",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|_| JobError::Database)?;

    for row in rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let beneficiaries = row.2;
        let policy_name = row.3;
        let enc_owner_name = row.4;

        let owner_name = decrypt_owner_name(&state.config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let mut tx = state.db.begin().await.map_err(|_| JobError::Database)?;
        let updated = sqlx::query(
            "UPDATE inheritance.policies SET status = 'released', updated_at = now() WHERE policy_id = $1 AND status = 'release_ready'",
        )
        .bind(policy_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| JobError::Database)?;

        if updated.rows_affected() > 0 {
            let payload = serde_json::json!({
                "policy_id": policy_id,
                "status": "released",
            });
            audit::append_event(&mut tx, policy_id, "policy_released", &payload)
                .await
                .map_err(|_| JobError::Audit)?;
        }

        tx.commit().await.map_err(|_| JobError::Database)?;

        for email in extract_emails(&beneficiaries) {
            let params = serde_json::json!({
                "brand_name": state.config.brand_name.clone(),
                "app_url": state.config.app_url.clone(),
                "policy_id": policy_id,
                "owner_name": owner_name.clone(),
                "policy_name": policy_name.clone(),
            });
            let dedupe_key = format!("release-ready-{}-{}", policy_id, email);
            enqueue_notify(
                &state,
                NotifyJob {
                    policy_id: Some(policy_id),
                    email,
                    template_id: state.config.brevo_release_ready_template_id.clone(),
                    params,
                    dedupe_key,
                    attempts: 0,
                },
            )
            .await?;
        }
    }

    Ok(())
}

async fn enqueue_owner_reminders(
    pool: &PgPool,
    state: &AppState,
    now: DateTime<Utc>,
) -> Result<(), JobError> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, DateTime<Utc>, String, String)>(
        "SELECT p.policy_id, p.owner_id, p.cadence::text, p.pending_at, p.label, pr.enc_legal_name 
         FROM inheritance.policies p
         JOIN auth_ext.persons pr ON pr.user_id = p.owner_id
         WHERE p.status = 'active' AND p.pending_at IS NOT NULL AND p.is_deleted = false",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| JobError::Database)?;

    for row in rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let cadence_str = row.2;
        let pending_at = row.3;
        let policy_name = row.4;
        let enc_owner_name = row.5;

        let owner_name = decrypt_owner_name(&state.config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let cadence_days = cadence_days(&cadence_str);
        let early_offset = Duration::days((cadence_days as f64 * 0.2).ceil() as i64);
        let urgent_offset = Duration::days((cadence_days as f64 * 0.1).ceil() as i64);
        let daily_window = Duration::days(7.min(cadence_days as i64));

        if now >= pending_at - early_offset {
            enqueue_owner_notification(
                pool,
                state,
                policy_id,
                owner_id,
                &state.config.brevo_owner_reminder_early_template_id,
                "owner-early",
                now,
                pending_at,
                &owner_name,
                &policy_name,
            )
            .await?;
        }

        if now >= pending_at - urgent_offset {
            enqueue_owner_notification(
                pool,
                state,
                policy_id,
                owner_id,
                &state.config.brevo_owner_reminder_urgent_template_id,
                "owner-urgent",
                now,
                pending_at,
                &owner_name,
                &policy_name,
            )
            .await?;
        }

        if now >= pending_at - daily_window && now < pending_at {
            enqueue_owner_notification(
                pool,
                state,
                policy_id,
                owner_id,
                &state.config.brevo_owner_reminder_daily_template_id,
                "owner-daily",
                now,
                pending_at,
                &owner_name,
                &policy_name,
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
    owner_name: &str,
    policy_name: &str,
) -> Result<(), JobError> {
    if let Some(email) = fetch_user_email(pool, owner_id).await {
        let params = serde_json::json!({
            "brand_name": state.config.brand_name.clone(),
            "app_url": state.config.app_url.clone(),
            "policy_id": policy_id,
            "pending_at": pending_at,
            "owner_name": owner_name,
            "policy_name": policy_name,
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
    let mut notify_storage = state.notify_storage.clone();
    notify_storage
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
                .filter_map(|v| {
                    v.get("email")
                        .and_then(|e| e.as_str())
                        .map(|s| s.to_string())
                })
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

fn decrypt_owner_name(key_b64: &str, enc_name_b64: &str, owner_id: Uuid) -> anyhow::Result<String> {
    let key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_b64)?;
    let key = Key::from_slice(&key_bytes);
    let enc_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(enc_name_b64)?;
    let aad = owner_id.as_bytes();
    let name_bytes = decrypt(key, &enc_bytes, Some(aad)).map_err(|_| anyhow::anyhow!("Decryption failed"))?;
    Ok(String::from_utf8(name_bytes)?)
}
