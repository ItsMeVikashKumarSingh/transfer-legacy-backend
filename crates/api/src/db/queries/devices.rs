use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct DeviceRow {
    pub device_id: Uuid,
    pub user_id: Uuid,
    pub ed25519_pubkey: Vec<u8>,
    pub device_meta: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub last_seen_at: Option<chrono::DateTime<chrono::Utc>>,
}

pub async fn count_devices(pool: &PgPool, user_id: Uuid) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM auth_ext.devices WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn insert_device(
    pool: &PgPool,
    device_id: Uuid,
    user_id: Uuid,
    ed25519_pubkey: Vec<u8>,
    device_meta: Option<Value>,
    crypto_version: String,
    schema_version: i32,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO auth_ext.devices (device_id, user_id, ed25519_pubkey, device_meta, crypto_version, schema_version) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(device_id)
    .bind(user_id)
    .bind(ed25519_pubkey)
    .bind(device_meta)
    .bind(crypto_version)
    .bind(schema_version)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn list_devices(pool: &PgPool, user_id: Uuid) -> Result<Vec<DeviceRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Vec<u8>, Option<Value>, chrono::DateTime<chrono::Utc>, Option<chrono::DateTime<chrono::Utc>>)>(
        "SELECT device_id, user_id, ed25519_pubkey, device_meta, created_at, last_seen_at FROM auth_ext.devices WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(|r| DeviceRow {
        device_id: r.0,
        user_id: r.1,
        ed25519_pubkey: r.2,
        device_meta: r.3,
        created_at: r.4,
        last_seen_at: r.5,
    }).collect())
}

pub async fn revoke_device(pool: &PgPool, user_id: Uuid, device_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE auth_ext.devices SET is_deleted = true, deleted_at = now() WHERE user_id = $1 AND device_id = $2",
    )
    .bind(user_id)
    .bind(device_id)
    .execute(pool)
    .await?;

    Ok(())
}
