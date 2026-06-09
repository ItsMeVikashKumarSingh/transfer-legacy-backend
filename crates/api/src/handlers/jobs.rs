use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use base64::Engine as _;
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::ApiError;
use crate::notifications::resend::{send_notification, NotificationTemplate};
use crate::state::AppState;
use transfer_legacy_crypto_core::{
    aead::decrypt, hash::sha256, jcs::canonicalize,
};
use transfer_legacy_shared_types::AppError;

#[derive(Serialize)]
pub struct JobResult {
    success: bool,
    message: String,
}

// Security Check: Verify Bearer Token matches TL_CRON_SECRET
async fn verify_cron_secret(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let config = state.config().await;
    let expected = match config.tl_cron_secret.as_deref() {
        Some(s) if !s.is_empty() => s,
        _ => {
            // If secret is not configured, deny by default to protect endpoints
            tracing::error!("TL_CRON_SECRET is not configured on the server!");
            return Err(ApiError::App(AppError::Unauthorized));
        }
    };

    let auth_header = headers
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(ApiError::App(AppError::Unauthorized))?;

    if !auth_header.starts_with("Bearer ") {
        return Err(ApiError::App(AppError::Unauthorized));
    }

    let token = &auth_header[7..];
    if token != expected {
        return Err(ApiError::App(AppError::Unauthorized));
    }

    Ok(())
}

// Helper to check deduplication and send notification email securely
async fn send_cron_notification(
    pool: &PgPool,
    state: &AppState,
    policy_id: Option<Uuid>,
    recipient_email: &str,
    template: NotificationTemplate,
    dedupe_key: &str,
) -> Result<(), anyhow::Error> {
    let template_name = template.template_name();
    
    // Check & Insert Notification Log (Prevent duplicates)
    let res = sqlx::query(
        "INSERT INTO notify.notification_log (notification_id, policy_id, recipient_email, template_id, status, dedupe_key) 
         VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (dedupe_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(policy_id)
    .bind(recipient_email)
    .bind(&template_name)
    .bind("queued")
    .bind(dedupe_key)
    .execute(pool)
    .await?;

    if res.rows_affected() == 0 {
        // Already processed
        return Ok(());
    }

    // Process notification
    let config = state.config().await;
    match send_notification(&config, recipient_email, template).await {
        Ok(_) => {
            sqlx::query(
                "UPDATE notify.notification_log SET status = 'sent', sent_at = now() WHERE dedupe_key = $1",
            )
            .bind(dedupe_key)
            .execute(pool)
            .await?;
        }
        Err(e) => {
            let error_msg = e.to_string();
            sqlx::query(
                "UPDATE notify.notification_log SET status = 'failed', error_message = $2 WHERE dedupe_key = $1",
            )
            .bind(dedupe_key)
            .bind(&error_msg)
            .execute(pool)
            .await?;
            return Err(anyhow::anyhow!("Resend error: {}", error_msg));
        }
    }

    Ok(())
}

// 1. POST /v1/jobs/heartbeat-eval
pub async fn heartbeat_eval(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<JobResult>, ApiError> {
    verify_cron_secret(&state, &headers).await?;
    let now = Utc::now();
    let pool = &state.db;
    let config = state.config().await;

    // A. Enqueue owner reminders
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, DateTime<Utc>, String, String)>(
        "SELECT p.policy_id, p.owner_id, p.cadence::text, p.pending_at, p.label, pr.enc_legal_name 
         FROM inheritance.policies p
         JOIN auth_ext.persons pr ON pr.user_id = p.owner_id
         WHERE p.status = 'active' AND p.pending_at IS NOT NULL AND p.is_deleted = false",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let cadence_str = row.2;
        let pending_at = row.3;
        let policy_name = row.4;
        let enc_owner_name = row.5;

        let owner_name = decrypt_owner_name(&config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let cadence_days = match cadence_str.as_str() {
            "1w" => 7,
            "15d" => 15,
            "1m" => 30,
            "3m" => 90,
            _ => 30,
        };

        let early_offset = Duration::days((cadence_days as f64 * 0.2).ceil() as i64);
        let urgent_offset = Duration::days((cadence_days as f64 * 0.1).ceil() as i64);
        let daily_window = Duration::days(7.min(cadence_days as i64));

        let user_email = sqlx::query_scalar::<_, String>("SELECT email FROM auth.users WHERE id = $1")
            .bind(owner_id)
            .fetch_one(pool)
            .await
            .ok();

        if let Some(email) = user_email {
            if now >= pending_at - early_offset {
                let template = NotificationTemplate::OwnerReminder {
                    urgency: "early".to_string(),
                    owner_name: owner_name.clone(),
                    policy_name: policy_name.clone(),
                    grace_deadline: pending_at.to_rfc3339(),
                };
                let dedupe_key = format!("owner_early-{}-{}", policy_id, now.date_naive());
                let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
            }

            if now >= pending_at - urgent_offset {
                let template = NotificationTemplate::OwnerReminder {
                    urgency: "urgent".to_string(),
                    owner_name: owner_name.clone(),
                    policy_name: policy_name.clone(),
                    grace_deadline: pending_at.to_rfc3339(),
                };
                let dedupe_key = format!("owner_urgent-{}-{}", policy_id, now.date_naive());
                let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
            }

            if now >= pending_at - daily_window && now < pending_at {
                let template = NotificationTemplate::OwnerReminder {
                    urgency: "daily".to_string(),
                    owner_name: owner_name.clone(),
                    policy_name: policy_name.clone(),
                    grace_deadline: pending_at.to_rfc3339(),
                };
                let dedupe_key = format!("owner_daily-{}-{}", policy_id, now.date_naive());
                let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
            }
        }
    }

    // B. Transition active -> pending
    let mut tx = pool.begin().await.map_err(|_| ApiError::App(AppError::Internal))?;
    let pending_rows = sqlx::query_as::<_, (Uuid, Uuid, DateTime<Utc>, DateTime<Utc>, String, String)>(
        "UPDATE inheritance.policies p SET status = 'pending', updated_at = now() 
         FROM auth_ext.persons pr 
         WHERE p.status = 'active' AND p.pending_at <= now() AND p.is_deleted = false AND pr.user_id = p.owner_id
         RETURNING p.policy_id, p.owner_id, p.pending_at, p.grace_deadline, p.label, pr.enc_legal_name",
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in pending_rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let pending_at = row.2;
        let grace_deadline = row.3;
        let policy_name = row.4;
        let enc_owner_name = row.5;

        let owner_name = decrypt_owner_name(&config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let payload = serde_json::json!({
            "policy_id": policy_id,
            "status": "pending",
            "pending_at": pending_at,
            "grace_deadline": grace_deadline,
        });
        crate::services::audit::append_event(&mut tx, policy_id, "policy_pending", &payload, None, None)
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;

        if let Ok(Some(email)) = sqlx::query_scalar::<_, String>("SELECT email FROM auth.users WHERE id = $1")
            .bind(owner_id)
            .fetch_optional(tx.as_mut())
            .await
        {
            let template = NotificationTemplate::OwnerReminder {
                urgency: "daily".to_string(),
                owner_name: owner_name.clone(),
                policy_name: policy_name.clone(),
                grace_deadline: grace_deadline.to_rfc3339(),
            };
            let dedupe_key = format!("owner-pending-{}-{}", policy_id, now.date_naive());
            let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
        }
    }

    // C. Transition pending -> investigating
    let investigating_rows = sqlx::query_as::<_, (Uuid, Uuid, serde_json::Value, serde_json::Value, String, String)>(
        "UPDATE inheritance.policies p SET status = 'investigating', updated_at = now() 
         FROM auth_ext.persons pr 
         WHERE p.status = 'pending' AND p.grace_deadline <= now() AND p.is_deleted = false AND pr.user_id = p.owner_id
         RETURNING p.policy_id, p.owner_id, p.beneficiaries, p.approvers, p.label, pr.enc_legal_name",
    )
    .fetch_all(tx.as_mut())
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in investigating_rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let beneficiaries = row.2;
        let approvers = row.3;
        let policy_name = row.4;
        let enc_owner_name = row.5;

        let owner_name = decrypt_owner_name(&config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let payload = serde_json::json!({
            "policy_id": policy_id,
            "status": "investigating",
        });
        crate::services::audit::append_event(&mut tx, policy_id, "policy_investigating", &payload, None, None)
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;

        for email in extract_emails(&beneficiaries) {
            let template = NotificationTemplate::ClaimAvailable {
                owner_name: owner_name.clone(),
                policy_name: policy_name.clone(),
            };
            let dedupe_key = format!("beneficiary-investigating-{}-{}", policy_id, email);
            let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
        }

        for email in extract_emails(&approvers) {
            let template = NotificationTemplate::AttestationRequest {
                owner_name: owner_name.clone(),
                policy_name: policy_name.clone(),
            };
            let dedupe_key = format!("approver-investigating-{}-{}", policy_id, email);
            let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
        }
    }

    tx.commit().await.map_err(|_| ApiError::App(AppError::Internal))?;

    Ok(Json(JobResult {
        success: true,
        message: "Heartbeat evaluation run completed successfully.".to_string(),
    }))
}

// 2. POST /v1/jobs/audit-anchor
#[derive(Deserialize)]
pub struct AuditAnchorRequest {
    pub date: Option<NaiveDate>,
}

pub async fn audit_anchor(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<AuditAnchorRequest>,
) -> Result<Json<JobResult>, ApiError> {
    verify_cron_secret(&state, &headers).await?;
    let pool = &state.db;
    let config = state.config().await;
    
    let target_date = payload.date.unwrap_or_else(|| Utc::now().date_naive() - Duration::days(1));

    let entries: Vec<Vec<u8>> = sqlx::query_scalar(
        "SELECT event_hash FROM audit.events WHERE created_at::date = $1 ORDER BY created_at ASC",
    )
    .bind(target_date)
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    let encoded: Vec<String> = entries
        .iter()
        .map(|v| base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(v))
        .collect();

    let snapshot = serde_json::json!({
        "date": target_date.to_string(),
        "event_hashes": encoded,
        "count_audit_entries": entries.len(),
        "server_id": "transfer-legacy-backend",
    });

    let snapshot_bytes = canonicalize(&snapshot).map_err(|_| ApiError::App(AppError::Internal))?;
    let head_hash = sha256(&snapshot_bytes);
    
    // Sign using swappable signer!
    let signature = state.signer.sign_digest("tl-signing", &head_hash)
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

    let anchor = serde_json::json!({
        "date": target_date.to_string(),
        "head_hash": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(&head_hash),
        "count_audit_entries": entries.len(),
        "server_id": "transfer-legacy-backend",
        "signature": signature,
    });

    let anchor_bytes = canonicalize(&anchor).map_err(|_| ApiError::App(AppError::Internal))?;
    let key = format!("{}/anchor.json", target_date);
    
    crate::services::b2::upload_anchor(&config, &key, anchor_bytes)
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

    sqlx::query(
        "INSERT INTO audit.anchors (anchor_date, head_hash, entry_count, snapshot, signature) 
         VALUES ($1,$2,$3,$4,$5) ON CONFLICT (anchor_date) DO NOTHING",
    )
    .bind(target_date)
    .bind(head_hash)
    .bind(entries.len() as i32)
    .bind(snapshot)
    .bind(signature)
    .execute(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    Ok(Json(JobResult {
        success: true,
        message: format!("Audit anchor generated and archived for {}", target_date),
    }))
}

// 3. POST /v1/jobs/release-eval
pub async fn release_eval(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<JobResult>, ApiError> {
    verify_cron_secret(&state, &headers).await?;
    let pool = &state.db;

    let rows = sqlx::query_as::<_, (Uuid, serde_json::Value, Uuid)>(
        "SELECT p.policy_id, p.m_of_n, c.claim_id 
         FROM inheritance.policies p 
         JOIN inheritance.claims c ON c.policy_id = p.policy_id 
         WHERE p.policy_type = 'm_of_n' AND p.status = 'investigating' AND c.status = 'confirmed'",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in rows {
        let m = row.1.get("m").and_then(|v| v.as_i64()).unwrap_or(0);
        if m <= 0 {
            continue;
        }
        let count = sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*) FROM inheritance.attestations WHERE claim_id = $1",
        )
        .bind(row.2)
        .fetch_one(pool)
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

        if count >= m {
            let mut tx = pool.begin().await.map_err(|_| ApiError::App(AppError::Internal))?;
            sqlx::query(
                "UPDATE inheritance.policies SET status = 'release_ready', 
                 conflict_hold_until = now() + interval '48 hours', updated_at = now() 
                 WHERE policy_id = $1 AND status = 'investigating'",
            )
            .bind(row.0)
            .execute(tx.as_mut())
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;

            let payload = serde_json::json!({
                "policy_id": row.0,
                "claim_id": row.2,
                "attestation_count": count,
                "required": m,
            });
            crate::services::audit::append_event(&mut tx, row.0, "m_of_n_release_ready", &payload, None, None)
                .await
                .map_err(|_| ApiError::App(AppError::Internal))?;
            tx.commit().await.map_err(|_| ApiError::App(AppError::Internal))?;
        }
    }

    Ok(Json(JobResult {
        success: true,
        message: "Release conditions evaluation run completed.".to_string(),
    }))
}

// 4. POST /v1/jobs/conflict-check
pub async fn conflict_check(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<JobResult>, ApiError> {
    verify_cron_secret(&state, &headers).await?;
    let pool = &state.db;

    let conflicts = sqlx::query_as::<_, (Uuid, i64)>(
        "SELECT policy_id, COUNT(DISTINCT claimant_person_id) 
         FROM inheritance.claims WHERE status = 'confirmed' 
         GROUP BY policy_id HAVING COUNT(DISTINCT claimant_person_id) > 1",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in conflicts {
        let mut tx = pool.begin().await.map_err(|_| ApiError::App(AppError::Internal))?;
        let updated = sqlx::query(
            "UPDATE inheritance.policies SET status = 'conflict_pending', 
             conflict_hold_until = now() + interval '48 hours', updated_at = now() 
             WHERE policy_id = $1 AND status = 'release_ready'",
        )
        .bind(row.0)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

        if updated.rows_affected() > 0 {
            let details = serde_json::json!({
                "distinct_claimants": row.1,
            });
            sqlx::query(
                "INSERT INTO ops.conflict_records (conflict_id, policy_id, reason, details) 
                 VALUES ($1,$2,$3,$4)",
            )
            .bind(Uuid::new_v4())
            .bind(row.0)
            .bind("multiple_confirmed_claimants")
            .bind(details)
            .execute(tx.as_mut())
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;

            let payload = serde_json::json!({
                "policy_id": row.0,
                "reason": "multiple_confirmed_claimants",
            });
            crate::services::audit::append_event(&mut tx, row.0, "conflict_detected", &payload, None, None)
                .await
                .map_err(|_| ApiError::App(AppError::Internal))?;
        }

        tx.commit().await.map_err(|_| ApiError::App(AppError::Internal))?;
    }

    let manual_rows = sqlx::query_as::<_, (Uuid,)>(
        "SELECT policy_id FROM inheritance.policies 
         WHERE status = 'conflict_pending' AND conflict_hold_until <= now()",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in manual_rows {
        let mut tx = pool.begin().await.map_err(|_| ApiError::App(AppError::Internal))?;
        sqlx::query(
            "UPDATE inheritance.policies SET status = 'manual_review', updated_at = now() 
             WHERE policy_id = $1 AND status = 'conflict_pending'",
        )
        .bind(row.0)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

        sqlx::query("INSERT INTO ops.manual_reviews (review_id, policy_id) VALUES ($1,$2)")
            .bind(Uuid::new_v4())
            .bind(row.0)
            .execute(tx.as_mut())
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;

        let payload = serde_json::json!({
            "policy_id": row.0,
            "status": "manual_review",
        });
        crate::services::audit::append_event(&mut tx, row.0, "manual_review_opened", &payload, None, None)
            .await
            .map_err(|_| ApiError::App(AppError::Internal))?;
        tx.commit().await.map_err(|_| ApiError::App(AppError::Internal))?;
    }

    Ok(Json(JobResult {
        success: true,
        message: "Conflict detection check completed successfully.".to_string(),
    }))
}

// 5. POST /v1/jobs/release-delivery
pub async fn release_delivery(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<JobResult>, ApiError> {
    verify_cron_secret(&state, &headers).await?;
    let pool = &state.db;
    let config = state.config().await;

    let rows = sqlx::query_as::<_, (Uuid, Uuid, serde_json::Value, String, String)>(
        "SELECT p.policy_id, p.owner_id, p.beneficiaries, p.label, pr.enc_legal_name 
         FROM inheritance.policies p 
         JOIN auth_ext.persons pr ON pr.user_id = p.owner_id
         WHERE p.status = 'release_ready' AND p.conflict_hold_until <= now()",
    )
    .fetch_all(pool)
    .await
    .map_err(|_| ApiError::App(AppError::Internal))?;

    for row in rows {
        let policy_id = row.0;
        let owner_id = row.1;
        let beneficiaries = row.2;
        let policy_name = row.3;
        let enc_owner_name = row.4;

        let owner_name = decrypt_owner_name(&config.server_aead_key_b64, &enc_owner_name, owner_id)
            .unwrap_or_else(|_| "Policy Owner".to_string());

        let mut tx = pool.begin().await.map_err(|_| ApiError::App(AppError::Internal))?;
        let updated = sqlx::query(
            "UPDATE inheritance.policies SET status = 'released', updated_at = now() 
             WHERE policy_id = $1 AND status = 'release_ready'",
        )
        .bind(policy_id)
        .execute(tx.as_mut())
        .await
        .map_err(|_| ApiError::App(AppError::Internal))?;

        if updated.rows_affected() > 0 {
            let payload = serde_json::json!({
                "policy_id": policy_id,
                "status": "released",
            });
            crate::services::audit::append_event(&mut tx, policy_id, "policy_released", &payload, None, None)
                .await
                .map_err(|_| ApiError::App(AppError::Internal))?;
        }

        tx.commit().await.map_err(|_| ApiError::App(AppError::Internal))?;

        for email in extract_emails(&beneficiaries) {
            let template = NotificationTemplate::ReleaseReady {
                owner_name: owner_name.clone(),
                policy_name: policy_name.clone(),
            };
            let dedupe_key = format!("release-ready-{}-{}", policy_id, email);
            let _ = send_cron_notification(pool, &state, Some(policy_id), &email, template, &dedupe_key).await;
        }
    }

    Ok(Json(JobResult {
        success: true,
        message: "Release delivery check completed successfully.".to_string(),
    }))
}

// Helpers
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

fn decrypt_owner_name(key_b64: &str, enc_name_b64: &str, owner_id: Uuid) -> anyhow::Result<String> {
    let key_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(key_b64)?;
    let enc_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(enc_name_b64)?;
    let aad = owner_id.as_bytes();

    if enc_bytes.len() < 24 {
        return Err(anyhow::anyhow!("Invalid encrypted name length"));
    }

    let (nonce, ciphertext) = enc_bytes.split_at(24);
    let name_bytes = decrypt(&key_bytes, nonce, ciphertext, aad)
        .map_err(|_| anyhow::anyhow!("Decryption failed"))?;

    Ok(String::from_utf8(name_bytes)?)
}
