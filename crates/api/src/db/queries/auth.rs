use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub struct OpaqueRecordRow {
    pub user_id: Uuid,
    pub opaque_record: Vec<u8>,
    pub emk_blob: Vec<u8>,
    pub argon2_params: Value,
    pub ed25519_pubkey: Vec<u8>,
    pub x25519_pubkey: Vec<u8>,
    pub kyber768_pubkey: Vec<u8>,
    pub crypto_version: String,
    pub schema_version: i32,
}

pub async fn insert_person_and_link(
    tx: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    enc_legal_name: Vec<u8>,
    enc_email: Vec<u8>,
) -> Result<Uuid, sqlx::Error> {
    let person_id = Uuid::new_v4();

    sqlx::query(
        "INSERT INTO auth_ext.persons (person_id, enc_legal_name, enc_email) VALUES ($1, $2, $3)",
    )
    .bind(person_id)
    .bind(enc_legal_name)
    .bind(enc_email)
    .execute(tx.as_mut())
    .await?;

    sqlx::query("INSERT INTO auth_ext.person_user_links (person_id, user_id) VALUES ($1, $2)")
        .bind(person_id)
        .bind(user_id)
        .execute(tx.as_mut())
        .await?;

    Ok(person_id)
}

pub async fn insert_opaque_record(
    tx: &mut Transaction<'_, Postgres>,
    row: &OpaqueRecordRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"INSERT INTO auth_ext.opaque_records
        (user_id, opaque_record, emk_blob, argon2_params, ed25519_pubkey, x25519_pubkey, kyber768_pubkey, crypto_version, schema_version)
        VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9)"#,
    )
    .bind(row.user_id)
    .bind(&row.opaque_record)
    .bind(&row.emk_blob)
    .bind(&row.argon2_params)
    .bind(&row.ed25519_pubkey)
    .bind(&row.x25519_pubkey)
    .bind(&row.kyber768_pubkey)
    .bind(&row.crypto_version)
    .bind(row.schema_version)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

pub async fn fetch_opaque_record(
    pool: &PgPool,
    user_id: Uuid,
) -> Result<OpaqueRecordRow, sqlx::Error> {
    let row = sqlx::query_as::<_, (Uuid, Vec<u8>, Vec<u8>, Value, Vec<u8>, Vec<u8>, Vec<u8>, String, i32)>(
        r#"SELECT user_id, opaque_record, emk_blob, argon2_params, ed25519_pubkey, x25519_pubkey, kyber768_pubkey, crypto_version, schema_version
        FROM auth_ext.opaque_records WHERE user_id = $1"#,
    )
    .bind(user_id)
    .fetch_one(pool)
    .await?;

    Ok(OpaqueRecordRow {
        user_id: row.0,
        opaque_record: row.1,
        emk_blob: row.2,
        argon2_params: row.3,
        ed25519_pubkey: row.4,
        x25519_pubkey: row.5,
        kyber768_pubkey: row.6,
        crypto_version: row.7,
        schema_version: row.8,
    })
}

pub async fn update_opaque_record(
    tx: &mut Transaction<'_, Postgres>,
    row: &OpaqueRecordRow,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"UPDATE auth_ext.opaque_records SET
            opaque_record = $2,
            emk_blob = $3,
            argon2_params = $4,
            ed25519_pubkey = $5,
            x25519_pubkey = $6,
            kyber768_pubkey = $7,
            crypto_version = $8,
            schema_version = $9,
            updated_at = now()
        WHERE user_id = $1"#,
    )
    .bind(row.user_id)
    .bind(&row.opaque_record)
    .bind(&row.emk_blob)
    .bind(&row.argon2_params)
    .bind(&row.ed25519_pubkey)
    .bind(&row.x25519_pubkey)
    .bind(&row.kyber768_pubkey)
    .bind(&row.crypto_version)
    .bind(row.schema_version)
    .execute(tx.as_mut())
    .await?;

    Ok(())
}

