use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct StepUpRow {
    pub challenge_id: Uuid,
    pub user_id: Uuid,
    pub challenge_type: String,
    pub action: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
}

pub async fn create_stepup_challenge(
    pool: &PgPool,
    user_id: Uuid,
    challenge_type: &str,
    action: &str,
    expires_at: DateTime<Utc>,
) -> Result<Uuid, sqlx::Error> {
    let challenge_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO auth_ext.stepup_challenges (challenge_id, user_id, challenge_type, action, expires_at) VALUES ($1,$2,$3,$4,$5)",
    )
    .bind(challenge_id)
    .bind(user_id)
    .bind(challenge_type)
    .bind(action)
    .bind(expires_at)
    .execute(pool)
    .await?;
    Ok(challenge_id)
}

pub async fn fetch_stepup_challenge(pool: &PgPool, challenge_id: Uuid) -> Result<StepUpRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, String, String, DateTime<Utc>, Option<DateTime<Utc>>)>(
        "SELECT challenge_id, user_id, challenge_type, action, expires_at, consumed_at FROM auth_ext.stepup_challenges WHERE challenge_id = $1",
    )
    .bind(challenge_id)
    .fetch_one(pool)
    .await?;

    Ok(StepUpRow {
        challenge_id: row.0,
        user_id: row.1,
        challenge_type: row.2,
        action: row.3,
        expires_at: row.4,
        consumed_at: row.5,
    })
}

pub async fn consume_stepup_challenge(pool: &PgPool, challenge_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_ext.stepup_challenges SET consumed_at = now() WHERE challenge_id = $1",
    )
    .bind(challenge_id)
    .execute(pool)
    .await?;
    Ok(())
}
