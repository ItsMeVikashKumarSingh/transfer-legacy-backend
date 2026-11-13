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

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct UserNotificationRow {
    pub id: Uuid,
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub notification_type: String,
    pub icon: String,
    pub action_url: Option<String>,
    pub read: bool,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, sqlx::FromRow)]
pub struct NotificationPreferencesRow {
    pub user_id: Uuid,
    pub email_enabled: bool,
    pub push_enabled: bool,
    pub in_app_enabled: bool,
    pub security_alerts: bool,
    pub heartbeat_reminders: bool,
    pub guardian_signals: bool,
    pub vault_updates: bool,
    pub updated_at: DateTime<Utc>,
}

pub async fn list_user_notifications(
    pool: &PgPool,
    user_id: Uuid,
    limit: i64,
    offset: i64,
    unread_only: bool,
) -> Result<Vec<UserNotificationRow>, sqlx::Error> {
    if unread_only {
        sqlx::query_as::<_, UserNotificationRow>(
            r#"
            SELECT id, user_id, title, message, notification_type, icon, action_url, read, metadata, created_at, read_at
            FROM notify.user_notifications
            WHERE user_id = $1 AND read = false
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    } else {
        sqlx::query_as::<_, UserNotificationRow>(
            r#"
            SELECT id, user_id, title, message, notification_type, icon, action_url, read, metadata, created_at, read_at
            FROM notify.user_notifications
            WHERE user_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(user_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await
    }
}

pub async fn count_unread_notifications(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM notify.user_notifications WHERE user_id = $1 AND read = false",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row.0)
}

pub async fn create_user_notification(
    pool: &PgPool,
    user_id: Uuid,
    title: &str,
    message: &str,
    notification_type: &str,
    icon: &str,
    action_url: Option<&str>,
    metadata: Option<serde_json::Value>,
) -> Result<UserNotificationRow, sqlx::Error> {
    let meta = metadata.unwrap_or_else(|| serde_json::json!({}));
    sqlx::query_as::<_, UserNotificationRow>(
        r#"
        INSERT INTO notify.user_notifications (
            user_id, title, message, notification_type, icon, action_url, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING id, user_id, title, message, notification_type, icon, action_url, read, metadata, created_at, read_at
        "#,
    )
    .bind(user_id)
    .bind(title)
    .bind(message)
    .bind(notification_type)
    .bind(icon)
    .bind(action_url)
    .bind(meta)
    .fetch_one(pool)
    .await
}

pub async fn mark_notification_read(
    pool: &PgPool,
    notification_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notify.user_notifications SET read = true, read_at = now() WHERE id = $1 AND user_id = $2",
    )
    .bind(notification_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn mark_all_notifications_read(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE notify.user_notifications SET read = true, read_at = now() WHERE user_id = $1 AND read = false",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected())
}

pub async fn delete_user_notification(
    pool: &PgPool,
    notification_id: Uuid,
    user_id: Uuid,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM notify.user_notifications WHERE id = $1 AND user_id = $2",
    )
    .bind(notification_id)
    .bind(user_id)
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn get_or_create_notification_preferences(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<NotificationPreferencesRow, sqlx::Error> {
    let existing = sqlx::query_as::<_, NotificationPreferencesRow>(
        "SELECT user_id, email_enabled, push_enabled, in_app_enabled, security_alerts, heartbeat_reminders, guardian_signals, vault_updates, updated_at FROM notify.notification_preferences WHERE user_id = $1",
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    if let Some(pref) = existing {
        return Ok(pref);
    }

    sqlx::query_as::<_, NotificationPreferencesRow>(
        r#"
        INSERT INTO notify.notification_preferences (user_id)
        VALUES ($1)
        ON CONFLICT (user_id) DO UPDATE SET updated_at = now()
        RETURNING user_id, email_enabled, push_enabled, in_app_enabled, security_alerts, heartbeat_reminders, guardian_signals, vault_updates, updated_at
        "#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn update_notification_preferences(
    pool: &PgPool,
    user_id: Uuid,
    email_enabled: bool,
    push_enabled: bool,
    in_app_enabled: bool,
    security_alerts: bool,
    heartbeat_reminders: bool,
    guardian_signals: bool,
    vault_updates: bool,
) -> Result<NotificationPreferencesRow, sqlx::Error> {
    sqlx::query_as::<_, NotificationPreferencesRow>(
        r#"
        INSERT INTO notify.notification_preferences (
            user_id, email_enabled, push_enabled, in_app_enabled,
            security_alerts, heartbeat_reminders, guardian_signals, vault_updates, updated_at
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, now())
        ON CONFLICT (user_id) DO UPDATE SET
            email_enabled = EXCLUDED.email_enabled,
            push_enabled = EXCLUDED.push_enabled,
            in_app_enabled = EXCLUDED.in_app_enabled,
            security_alerts = EXCLUDED.security_alerts,
            heartbeat_reminders = EXCLUDED.heartbeat_reminders,
            guardian_signals = EXCLUDED.guardian_signals,
            vault_updates = EXCLUDED.vault_updates,
            updated_at = now()
        RETURNING user_id, email_enabled, push_enabled, in_app_enabled, security_alerts, heartbeat_reminders, guardian_signals, vault_updates, updated_at
        "#,
    )
    .bind(user_id)
    .bind(email_enabled)
    .bind(push_enabled)
    .bind(in_app_enabled)
    .bind(security_alerts)
    .bind(heartbeat_reminders)
    .bind(guardian_signals)
    .bind(vault_updates)
    .fetch_one(pool)
    .await
}
