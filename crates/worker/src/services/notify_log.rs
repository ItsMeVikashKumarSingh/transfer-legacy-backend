use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum NotifyLogError {
    #[error("db error")]
    Database,
}

pub async fn insert_queued(
    pool: &PgPool,
    policy_id: Option<Uuid>,
    recipient_email: &str,
    template_id: &str,
    dedupe_key: &str,
) -> Result<bool, NotifyLogError> {
    let res = sqlx::query(
        "INSERT INTO notify.notification_log (notification_id, policy_id, recipient_email, template_id, status, dedupe_key) VALUES ($1,$2,$3,$4,$5,$6) ON CONFLICT (dedupe_key) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(policy_id)
    .bind(recipient_email)
    .bind(template_id)
    .bind("queued")
    .bind(dedupe_key)
    .execute(pool)
    .await
    .map_err(|_| NotifyLogError::Database)?;

    Ok(res.rows_affected() > 0)
}

pub async fn mark_sent(pool: &PgPool, dedupe_key: &str) -> Result<(), NotifyLogError> {
    sqlx::query(
        "UPDATE notify.notification_log SET status = 'sent', sent_at = now() WHERE dedupe_key = $1",
    )
    .bind(dedupe_key)
    .execute(pool)
    .await
    .map_err(|_| NotifyLogError::Database)?;
    Ok(())
}

pub async fn mark_failed(
    pool: &PgPool,
    dedupe_key: &str,
    error_message: &str,
) -> Result<(), NotifyLogError> {
    sqlx::query(
        "UPDATE notify.notification_log SET status = 'failed', error_message = $2 WHERE dedupe_key = $1",
    )
    .bind(dedupe_key)
    .bind(error_message)
    .execute(pool)
    .await
    .map_err(|_| NotifyLogError::Database)?;
    Ok(())
}
