use sqlx::PgPool;
use uuid::Uuid;

pub async fn insert_totp_factor(
    pool: &PgPool,
    user_id: Uuid,
    secret_enc: Vec<u8>,
    crypto_version: String,
    schema_version: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO auth_ext.mfa_factors (user_id, factor_type, totp_secret_enc, crypto_version, schema_version) VALUES ($1, 'totp', $2, $3, $4)",
    )
    .bind(user_id)
    .bind(secret_enc)
    .bind(crypto_version)
    .bind(schema_version)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn fetch_totp_secret(pool: &PgPool, user_id: Uuid) -> Result<Vec<u8>, sqlx::Error> {
    let secret = sqlx::query_scalar::<_, Vec<u8>>(
        "SELECT totp_secret_enc FROM auth_ext.mfa_factors WHERE user_id = $1 AND factor_type = 'totp' AND enabled = true AND is_deleted = false ORDER BY created_at DESC LIMIT 1",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(secret)
}
