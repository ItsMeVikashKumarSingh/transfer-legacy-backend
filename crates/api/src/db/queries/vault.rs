use serde_json::Value;
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug)]
pub struct VaultItemRow {
    pub item_id: Uuid,
    pub user_id: Uuid,
    pub ciphertext: Vec<u8>,
    pub item_meta: Option<Value>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug)]
pub struct VaultShareRow {
    pub share_id: Uuid,
    pub item_id: Uuid,
    pub owner_id: Uuid,
    pub grantee_id: Uuid,
    pub envelope: Vec<u8>,
    pub grant_sig: Vec<u8>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub async fn insert_item(
    pool: &PgPool,
    user_id: Uuid,
    ciphertext: Vec<u8>,
    item_meta: Option<Value>,
    crypto_version: String,
    schema_version: i32,
) -> Result<Uuid, sqlx::Error> {
    let item_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.items (item_id, user_id, ciphertext, item_meta, crypto_version, schema_version) VALUES ($1,$2,$3,$4,$5,$6)",
    )
    .bind(item_id)
    .bind(user_id)
    .bind(ciphertext)
    .bind(item_meta)
    .bind(crypto_version)
    .bind(schema_version)
    .execute(pool)
    .await?;

    Ok(item_id)
}

pub async fn list_items(pool: &PgPool, user_id: Uuid) -> Result<Vec<VaultItemRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Vec<u8>, Option<Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT item_id, user_id, ciphertext, item_meta, created_at FROM core.items WHERE user_id = $1 AND is_deleted = false",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| VaultItemRow {
            item_id: r.0,
            user_id: r.1,
            ciphertext: r.2,
            item_meta: r.3,
            created_at: r.4,
        })
        .collect())
}

pub async fn get_item(
    pool: &PgPool,
    user_id: Uuid,
    item_id: Uuid,
) -> Result<VaultItemRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Uuid, Vec<u8>, Option<Value>, chrono::DateTime<chrono::Utc>)>(
        "SELECT item_id, user_id, ciphertext, item_meta, created_at FROM core.items WHERE user_id = $1 AND item_id = $2 AND is_deleted = false",
    )
    .bind(user_id)
    .bind(item_id)
    .fetch_one(pool)
    .await?;

    Ok(VaultItemRow {
        item_id: row.0,
        user_id: row.1,
        ciphertext: row.2,
        item_meta: row.3,
        created_at: row.4,
    })
}

pub async fn delete_item(pool: &PgPool, user_id: Uuid, item_id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE core.items SET is_deleted = true, deleted_at = now() WHERE user_id = $1 AND item_id = $2",
    )
    .bind(user_id)
    .bind(item_id)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_item(
    pool: &PgPool,
    user_id: Uuid,
    item_id: Uuid,
    ciphertext: Vec<u8>,
    item_meta: Option<Value>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE core.items SET ciphertext = $1, item_meta = $2, version = version + 1, updated_at = now() WHERE user_id = $3 AND item_id = $4 AND is_deleted = false",
    )
    .bind(ciphertext)
    .bind(item_meta)
    .bind(user_id)
    .bind(item_id)
    .execute(pool)
    .await?;
    Ok(())
}


pub async fn insert_share(
    pool: &PgPool,
    owner_id: Uuid,
    item_id: Uuid,
    grantee_id: Uuid,
    envelope: Vec<u8>,
    grant_sig: Vec<u8>,
    crypto_version: String,
    schema_version: i32,
) -> Result<Uuid, sqlx::Error> {
    let share_id = Uuid::new_v4();
    sqlx::query(
        "INSERT INTO core.shares (share_id, item_id, owner_id, grantee_id, envelope, grant_sig, crypto_version, schema_version) VALUES ($1,$2,$3,$4,$5,$6,$7,$8)",
    )
    .bind(share_id)
    .bind(item_id)
    .bind(owner_id)
    .bind(grantee_id)
    .bind(envelope)
    .bind(grant_sig)
    .bind(crypto_version)
    .bind(schema_version)
    .execute(pool)
    .await?;

    Ok(share_id)
}

pub async fn list_shares(pool: &PgPool, owner_id: Uuid) -> Result<Vec<VaultShareRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid, Vec<u8>, Vec<u8>, chrono::DateTime<chrono::Utc>)>(
        "SELECT share_id, item_id, owner_id, grantee_id, envelope, grant_sig, created_at FROM core.shares WHERE owner_id = $1 AND is_deleted = false",
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| VaultShareRow {
            share_id: r.0,
            item_id: r.1,
            owner_id: r.2,
            grantee_id: r.3,
            envelope: r.4,
            grant_sig: r.5,
            created_at: r.6,
        })
        .collect())
}

pub async fn list_shares_for_grantee_owner(
    pool: &PgPool,
    owner_id: Uuid,
    grantee_id: Uuid,
) -> Result<Vec<VaultShareRow>, sqlx::Error> {
    let rows = sqlx::query_as::<_, (Uuid, Uuid, Uuid, Uuid, Vec<u8>, Vec<u8>, chrono::DateTime<chrono::Utc>)>(
        "SELECT share_id, item_id, owner_id, grantee_id, envelope, grant_sig, created_at FROM core.shares WHERE owner_id = $1 AND grantee_id = $2 AND is_deleted = false",
    )
    .bind(owner_id)
    .bind(grantee_id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| VaultShareRow {
            share_id: r.0,
            item_id: r.1,
            owner_id: r.2,
            grantee_id: r.3,
            envelope: r.4,
            grant_sig: r.5,
            created_at: r.6,
        })
        .collect())
}

pub async fn revoke_share(
    pool: &PgPool,
    owner_id: Uuid,
    share_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE core.shares SET is_deleted = true, deleted_at = now() WHERE owner_id = $1 AND share_id = $2",
    )
    .bind(owner_id)
    .bind(share_id)
    .execute(pool)
    .await?;
    Ok(())
}
