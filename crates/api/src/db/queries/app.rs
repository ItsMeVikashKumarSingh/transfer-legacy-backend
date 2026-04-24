use sqlx::{Postgres, Pool};
use transfer_legacy_shared_types::models::app::{BrandingConfig, AppContent, WaitlistEntry, WaitlistSignupRequest};
use uuid::Uuid;

pub async fn fetch_branding(pool: &Pool<Postgres>) -> Result<BrandingConfig, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT brand_name, logo_url, support_email, support_phone, support_address, theme_config FROM app.settings WHERE id = 1",
    )
    .fetch_one(pool)
    .await?;

    Ok(BrandingConfig {
        brand_name: row.get("brand_name"),
        logo_url: row.get("logo_url"),
        support_email: row.get("support_email"),
        support_phone: row.get("support_phone"),
        support_address: row.get("support_address"),
        theme_config: row.get("theme_config"),
    })
}

pub async fn update_branding(
    pool: &Pool<Postgres>,
    config: BrandingConfig,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE app.settings SET brand_name = $1, logo_url = $2, support_email = $3, support_phone = $4, support_address = $5, theme_config = $6, updated_at = now() WHERE id = 1",
    )
    .bind(config.brand_name)
    .bind(config.logo_url)
    .bind(config.support_email)
    .bind(config.support_phone)
    .bind(config.support_address)
    .bind(config.theme_config)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn fetch_app_content(pool: &Pool<Postgres>, slug: &str) -> Result<AppContent, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT slug, body, version FROM app.content WHERE slug = $1 AND is_deleted = false",
    )
    .bind(slug)
    .fetch_one(pool)
    .await?;

    Ok(AppContent {
        slug: row.get("slug"),
        body: row.get("body"),
        version: row.get("version"),
    })
}

pub async fn update_app_content(
    pool: &Pool<Postgres>,
    content: AppContent,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO app.content (slug, body, version, updated_at) VALUES ($1, $2, $3, now()) ON CONFLICT (slug) DO UPDATE SET body = EXCLUDED.body, version = EXCLUDED.version, updated_at = now()",
    )
    .bind(content.slug)
    .bind(content.body)
    .bind(content.version)
    .execute(pool)
    .await?;

    Ok(())
}

pub async fn insert_waitlist_signup(
    pool: &Pool<Postgres>,
    req: WaitlistSignupRequest,
) -> Result<Uuid, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "INSERT INTO app.waitlist (email, name, meta) VALUES ($1, $2, $3) ON CONFLICT (email) DO UPDATE SET updated_at = now() RETURNING id",
    )
    .bind(req.email)
    .bind(req.name)
    .bind(req.metadata)
    .fetch_one(pool)
    .await?;

    Ok(row.get::<Uuid, _>("id"))
}

pub async fn list_waitlist_entries(pool: &Pool<Postgres>) -> Result<Vec<WaitlistEntry>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, email, name, meta, created_at FROM app.waitlist WHERE is_deleted = false ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    let entries = rows
        .into_iter()
        .map(|r| WaitlistEntry {
            id: r.get("id"),
            email: r.get("email"),
            name: r.get("name"),
            meta: r.get("meta"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(entries)
}
