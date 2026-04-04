use serde::Serialize;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum DlqError {
    #[error("db error")]
    Database,
}

pub async fn record_failed_job<T: Serialize>(
    pool: &PgPool,
    job_type: &str,
    payload: &T,
    error_message: &str,
    attempts: i32,
) -> Result<(), DlqError> {
    let payload_json = serde_json::to_value(payload).unwrap_or_else(|_| serde_json::json!({}));
    sqlx::query(
        "INSERT INTO ops.failed_jobs (job_id, job_type, payload, error_message, attempts) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(Uuid::new_v4())
    .bind(job_type)
    .bind(payload_json)
    .bind(error_message)
    .bind(attempts)
    .execute(pool)
    .await
    .map_err(|_| DlqError::Database)?;
    Ok(())
}
