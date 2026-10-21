use chrono::{DateTime, Utc};
use metrics::histogram;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
use transfer_legacy_shared_types::models::ops::{OpsCoupon, OpsPlan, OpsUser};
use uuid::Uuid;

#[derive(Debug)]
pub struct ManualReviewRow {
    pub review_id: Uuid,
    pub policy_id: Uuid,
    pub conflict_id: Option<Uuid>,
    pub status: String,
    pub notes: Option<Value>,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

pub async fn list_manual_reviews(
    pool: &PgPool,
    status: Option<&str>,
) -> Result<Vec<ManualReviewRow>, sqlx::Error> {
    let started = Instant::now();
    let rows = if let Some(status) = status {
        sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, Option<Value>, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT review_id, policy_id, conflict_id, status::text, notes, created_at, resolved_at FROM ops.manual_reviews WHERE status::text = $1 ORDER BY created_at DESC",
        )
        .bind(status)
        .fetch_all(pool)
        .await?
    } else {
        sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, Option<Value>, DateTime<Utc>, Option<DateTime<Utc>>)>(
            "SELECT review_id, policy_id, conflict_id, status::text, notes, created_at, resolved_at FROM ops.manual_reviews ORDER BY created_at DESC",
        )
        .fetch_all(pool)
        .await?
    };

    let result = rows
        .into_iter()
        .map(|r| ManualReviewRow {
            review_id: r.0,
            policy_id: r.1,
            conflict_id: r.2,
            status: r.3,
            notes: r.4,
            created_at: r.5,
            resolved_at: r.6,
        })
        .collect();
    histogram!("db_query_duration_seconds", "query" => "ops.list_manual_reviews")
        .record(started.elapsed().as_secs_f64());
    Ok(result)
}

pub async fn fetch_manual_review_for_update(
    tx: &mut Transaction<'_, Postgres>,
    review_id: Uuid,
) -> Result<ManualReviewRow, sqlx::Error> {
    let started = Instant::now();
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, Option<Value>, DateTime<Utc>, Option<DateTime<Utc>>)>(
        "SELECT review_id, policy_id, conflict_id, status::text, notes, created_at, resolved_at FROM ops.manual_reviews WHERE review_id = $1 FOR UPDATE",
    )
    .bind(review_id)
    .fetch_one(tx.as_mut())
    .await?;

    let result = ManualReviewRow {
        review_id: row.0,
        policy_id: row.1,
        conflict_id: row.2,
        status: row.3,
        notes: row.4,
        created_at: row.5,
        resolved_at: row.6,
    };
    histogram!("db_query_duration_seconds", "query" => "ops.fetch_manual_review_for_update")
        .record(started.elapsed().as_secs_f64());
    Ok(result)
}

pub async fn fetch_manual_review(
    pool: &PgPool,
    review_id: Uuid,
) -> Result<ManualReviewRow, sqlx::Error> {
    let started = Instant::now();
    let row = sqlx::query_as::<_, (Uuid, Uuid, Option<Uuid>, String, Option<Value>, DateTime<Utc>, Option<DateTime<Utc>>)>(
        "SELECT review_id, policy_id, conflict_id, status::text, notes, created_at, resolved_at FROM ops.manual_reviews WHERE review_id = $1",
    )
    .bind(review_id)
    .fetch_one(pool)
    .await?;

    let result = ManualReviewRow {
        review_id: row.0,
        policy_id: row.1,
        conflict_id: row.2,
        status: row.3,
        notes: row.4,
        created_at: row.5,
        resolved_at: row.6,
    };
    histogram!("db_query_duration_seconds", "query" => "ops.fetch_manual_review")
        .record(started.elapsed().as_secs_f64());
    Ok(result)
}

pub async fn update_manual_review_decision(
    tx: &mut Transaction<'_, Postgres>,
    review_id: Uuid,
    decision: &str,
    notes: Value,
) -> Result<(), sqlx::Error> {
    let started = Instant::now();
    sqlx::query(
        "UPDATE ops.manual_reviews SET status = 'resolved', notes = $1, resolved_at = now() WHERE review_id = $2",
    )
    .bind(notes)
    .bind(review_id)
    .execute(tx.as_mut())
    .await?;

    if decision == "released" {
        sqlx::query(
            "UPDATE inheritance.policies SET status = 'released', updated_at = now() WHERE policy_id = (SELECT policy_id FROM ops.manual_reviews WHERE review_id = $1)",
        )
        .bind(review_id)
        .execute(tx.as_mut())
        .await?;
    } else if decision == "cancelled" {
        sqlx::query(
            "UPDATE inheritance.policies SET status = 'cancelled', updated_at = now() WHERE policy_id = (SELECT policy_id FROM ops.manual_reviews WHERE review_id = $1)",
        )
        .bind(review_id)
        .execute(tx.as_mut())
        .await?;
    }

    histogram!("db_query_duration_seconds", "query" => "ops.update_manual_review_decision")
        .record(started.elapsed().as_secs_f64());
    Ok(())
}

// --- Admin & Role Management ---

use transfer_legacy_shared_types::models::ops::{OpsAdmin, OpsRole};

pub async fn fetch_admin_by_email(
    pool: &PgPool,
    email: &str,
) -> Result<(Uuid, String, String), sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "SELECT a.id, a.password_hash, r.name as role_name 
         FROM ops.admins a 
         JOIN ops.roles r ON a.role_id = r.id 
         WHERE a.email = $1 AND a.is_active = true",
    )
    .bind(email)
    .fetch_one(pool)
    .await?;

    Ok((
        row.get("id"),
        row.get("password_hash"),
        row.get("role_name"),
    ))
}

pub async fn list_admins(pool: &PgPool) -> Result<Vec<OpsAdmin>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT a.id, a.email, a.role_id, r.name as role_name, a.is_active, a.last_login, a.created_at 
         FROM ops.admins a 
         JOIN ops.roles r ON a.role_id = r.id 
         ORDER BY a.created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OpsAdmin {
            id: r.get("id"),
            email: r.get("email"),
            role_id: r.get("role_id"),
            role_name: r.get("role_name"),
            is_active: r.get("is_active"),
            last_login: r.get("last_login"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub async fn create_admin(
    pool: &PgPool,
    email: &str,
    password_hash: &str,
    role_id: Uuid,
) -> Result<Uuid, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "INSERT INTO ops.admins (email, password_hash, role_id) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(email)
    .bind(password_hash)
    .bind(role_id)
    .fetch_one(pool)
    .await?;

    Ok(row.get("id"))
}

pub async fn delete_admin(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ops.admins WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_admin_password(
    pool: &PgPool,
    id: Uuid,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE ops.admins SET password_hash = $1, updated_at = now() WHERE id = $2")
        .bind(password_hash)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_last_login(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE ops.admins SET last_login = now() WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_roles(pool: &PgPool) -> Result<Vec<OpsRole>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query("SELECT id, name, description, permissions FROM ops.roles")
        .fetch_all(pool)
        .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let permissions_val: Value = r.get("permissions");
            let permissions: Vec<String> =
                serde_json::from_value(permissions_val).unwrap_or_else(|_| vec![]);

            OpsRole {
                id: r.get("id"),
                name: r.get("name"),
                description: r.get("description"),
                permissions,
            }
        })
        .collect())
}

pub async fn fetch_role_permissions_by_name(
    pool: &PgPool,
    role_name: &str,
) -> Result<Vec<String>, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query("SELECT permissions FROM ops.roles WHERE name = $1")
        .bind(role_name)
        .fetch_one(pool)
        .await?;

    Ok(row.get("permissions"))
}

pub async fn get_admin_password_hash(pool: &PgPool, id: Uuid) -> Result<String, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query("SELECT password_hash FROM ops.admins WHERE id = $1")
        .bind(id)
        .fetch_one(pool)
        .await?;
    Ok(row.get("password_hash"))
}

pub async fn create_role(
    pool: &PgPool,
    name: &str,
    description: Option<&str>,
    permissions: Vec<String>,
) -> Result<Uuid, sqlx::Error> {
    use sqlx::Row;
    let row = sqlx::query(
        "INSERT INTO ops.roles (name, description, permissions) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(name)
    .bind(description)
    .bind(serde_json::to_value(permissions).unwrap_or(serde_json::Value::Array(vec![])))
    .fetch_one(pool)
    .await?;

    Ok(row.get("id"))
}

pub async fn delete_role(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ops.roles WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn update_role(
    pool: &PgPool,
    id: Uuid,
    name: &str,
    description: Option<&str>,
    permissions: Vec<String>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE ops.roles SET name = $1, description = $2, permissions = $3, updated_at = now() WHERE id = $4"
    )
    .bind(name)
    .bind(description)
    .bind(serde_json::to_value(permissions).unwrap_or(serde_json::Value::Array(vec![])))
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

// --- Activity Logging ---

pub async fn log_activity(
    pool: &PgPool,
    admin_id: Option<Uuid>,
    action: &str,
    entity_type: Option<&str>,
    entity_id: Option<&str>,
    metadata: Option<Value>,
    ip_address: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO ops.activity_logs (admin_id, action, entity_type, entity_id, metadata, ip_address) 
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(admin_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .bind(metadata)
    .bind(ip_address)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_activity_logs(
    pool: &PgPool,
    action_filter: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>, sqlx::Error> {
    let query = if let Some(action) = action_filter {
        sqlx::query(
            "SELECT l.id, l.admin_id, a.email as admin_email, l.action, l.entity_type, l.entity_id, l.metadata, l.ip_address, l.created_at 
             FROM ops.activity_logs l 
             LEFT JOIN ops.admins a ON l.admin_id = a.id 
             WHERE l.action = $1 
             ORDER BY l.created_at DESC 
             LIMIT $2"
        )
        .bind(action)
        .bind(limit)
    } else {
        sqlx::query(
            "SELECT l.id, l.admin_id, a.email as admin_email, l.action, l.entity_type, l.entity_id, l.metadata, l.ip_address, l.created_at 
             FROM ops.activity_logs l 
             LEFT JOIN ops.admins a ON l.admin_id = a.id 
             ORDER BY l.created_at DESC 
             LIMIT $1"
        )
        .bind(limit)
    };

    let rows = query.fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            use sqlx::Row;
            serde_json::json!({
                "id": r.get::<Uuid, _>("id"),
                "admin_id": r.get::<Option<Uuid>, _>("admin_id"),
                "admin_email": r.get::<Option<String>, _>("admin_email"),
                "action": r.get::<String, _>("action"),
                "entity_type": r.get::<Option<String>, _>("entity_type"),
                "entity_id": r.get::<Option<String>, _>("entity_id"),
                "metadata": r.get::<Option<Value>, _>("metadata"),
                "ip_address": r.get::<Option<String>, _>("ip_address"),
                "created_at": r.get::<DateTime<Utc>, _>("created_at"),
            })
        })
        .collect())
}

pub async fn list_ops_users(pool: &PgPool) -> Result<Vec<OpsUser>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        r#"
        SELECT 
            u.id,
            COALESCE(u.email, '') as email,
            COALESCE(m.plan_tier, 'Personal') as plan,
            COALESCE(m.status, 'active') as status,
            COALESCE(v.vault_count, 0) as vault_assets_count,
            COALESCE(p.guardian_count, 0) as guardians_count,
            CASE 
                WHEN pol.last_heartbeat_at IS NOT NULL 
                THEN GREATEST(0, EXTRACT(DAY FROM (now() - pol.last_heartbeat_at))::int)
                ELSE 0
            END as heartbeat_streak_days,
            m.last_active_at as last_active,
            COALESCE(u.created_at, now()) as created_at
        FROM auth.users u
        LEFT JOIN ops.user_metadata m ON u.id = m.user_id
        LEFT JOIN (
            SELECT user_id, COUNT(*) as vault_count 
            FROM core.items 
            WHERE is_deleted = false 
            GROUP BY user_id
        ) v ON u.id = v.user_id
        LEFT JOIN (
            SELECT owner_id, COUNT(*) as guardian_count 
            FROM inheritance.policies 
            WHERE is_deleted = false 
            GROUP BY owner_id
        ) p ON u.id = p.owner_id
        LEFT JOIN (
            SELECT owner_id, MAX(last_heartbeat_at) as last_heartbeat_at
            FROM inheritance.policies
            WHERE is_deleted = false
            GROUP BY owner_id
        ) pol ON u.id = pol.owner_id
        ORDER BY u.created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OpsUser {
            id: r.get("id"),
            email: r.get("email"),
            plan: r.get("plan"),
            status: r.get("status"),
            vault_assets_count: r.get("vault_assets_count"),
            guardians_count: r.get("guardians_count"),
            heartbeat_streak_days: r.get("heartbeat_streak_days"),
            last_active: r.get("last_active"),
            created_at: r.get("created_at"),
        })
        .collect())
}

pub async fn update_user_status(
    pool: &PgPool,
    user_id: Uuid,
    status: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO ops.user_metadata (user_id, status, updated_at)
        VALUES ($1, $2, now())
        ON CONFLICT (user_id) DO UPDATE SET
            status = EXCLUDED.status,
            updated_at = now()
        "#
    )
    .bind(user_id)
    .bind(status)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn update_user_plan(pool: &PgPool, user_id: Uuid, plan: &str) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO ops.user_metadata (user_id, plan_tier, updated_at)
        VALUES ($1, $2, now())
        ON CONFLICT (user_id) DO UPDATE SET
            plan_tier = EXCLUDED.plan_tier,
            updated_at = now()
        "#
    )
    .bind(user_id)
    .bind(plan)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_plans(pool: &PgPool) -> Result<Vec<OpsPlan>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, name, monthly_price::float8 as monthly_price, yearly_price::float8 as yearly_price, description, features, max_guardians, max_assets, highlighted, active, created_at, updated_at FROM ops.plans ORDER BY monthly_price ASC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| {
            let features_val: serde_json::Value = r.get("features");
            let features: Vec<String> = serde_json::from_value(features_val).unwrap_or_default();
            OpsPlan {
                id: r.get("id"),
                name: r.get("name"),
                monthly_price: r.get("monthly_price"),
                yearly_price: r.get("yearly_price"),
                description: r.get("description"),
                features,
                max_guardians: r.get("max_guardians"),
                max_assets: r.get("max_assets"),
                highlighted: r.get("highlighted"),
                active: r.get("active"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
            }
        })
        .collect())
}

#[allow(clippy::too_many_arguments)]
pub async fn upsert_plan(
    pool: &PgPool,
    id: &str,
    name: &str,
    monthly_price: f64,
    yearly_price: f64,
    description: &str,
    features: &[String],
    max_guardians: i32,
    max_assets: i32,
    highlighted: bool,
    active: bool,
) -> Result<(), sqlx::Error> {
    let features_json = serde_json::to_value(features).unwrap_or(serde_json::json!([]));
    sqlx::query(
        r#"
        INSERT INTO ops.plans (id, name, monthly_price, yearly_price, description, features, max_guardians, max_assets, highlighted, active, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, now())
        ON CONFLICT (id) DO UPDATE SET
            name = EXCLUDED.name,
            monthly_price = EXCLUDED.monthly_price,
            yearly_price = EXCLUDED.yearly_price,
            description = EXCLUDED.description,
            features = EXCLUDED.features,
            max_guardians = EXCLUDED.max_guardians,
            max_assets = EXCLUDED.max_assets,
            highlighted = EXCLUDED.highlighted,
            active = EXCLUDED.active,
            updated_at = now()
        "#
    )
    .bind(id)
    .bind(name)
    .bind(monthly_price)
    .bind(yearly_price)
    .bind(description)
    .bind(features_json)
    .bind(max_guardians)
    .bind(max_assets)
    .bind(highlighted)
    .bind(active)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_plan(pool: &PgPool, id: &str) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ops.plans WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn list_coupons(pool: &PgPool) -> Result<Vec<OpsCoupon>, sqlx::Error> {
    use sqlx::Row;
    let rows = sqlx::query(
        "SELECT id, code, discount_percent, max_uses, current_uses, expires_at, active, description, created_at, updated_at FROM ops.coupons ORDER BY created_at DESC"
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|r| OpsCoupon {
            id: r.get("id"),
            code: r.get("code"),
            discount_percent: r.get("discount_percent"),
            max_uses: r.get("max_uses"),
            current_uses: r.get("current_uses"),
            expires_at: r.get("expires_at"),
            active: r.get("active"),
            description: r.get("description"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
        .collect())
}

pub async fn create_coupon(
    pool: &PgPool,
    code: &str,
    discount_percent: i32,
    max_uses: i32,
    expires_at: DateTime<Utc>,
    description: Option<&str>,
) -> Result<Uuid, sqlx::Error> {
    let code_upper = code.trim().to_uppercase();
    let row = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO ops.coupons (code, discount_percent, max_uses, expires_at, description)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING id
        "#,
    )
    .bind(code_upper)
    .bind(discount_percent)
    .bind(max_uses)
    .bind(expires_at)
    .bind(description)
    .fetch_one(pool)
    .await?;
    Ok(row)
}

pub async fn toggle_coupon_active(pool: &PgPool, id: Uuid) -> Result<bool, sqlx::Error> {
    let new_active = sqlx::query_scalar::<_, bool>(
        "UPDATE ops.coupons SET active = NOT active, updated_at = now() WHERE id = $1 RETURNING active",
    )
    .bind(id)
    .fetch_one(pool)
    .await?;
    Ok(new_active)
}

pub async fn delete_coupon(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM ops.coupons WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn validate_coupon_db(
    pool: &PgPool,
    code: &str,
    amount: f64,
) -> Result<(bool, i32, f64, f64, String), sqlx::Error> {
    use sqlx::Row;
    let code_upper = code.trim().to_uppercase();
    let opt = sqlx::query(
        "SELECT discount_percent, max_uses, current_uses, expires_at, active FROM ops.coupons WHERE code = $1",
    )
    .bind(&code_upper)
    .fetch_optional(pool)
    .await?;

    let Some(row) = opt else {
        return Ok((false, 0, 0.0, amount, "Coupon code not found".to_string()));
    };

    let active: bool = row.get("active");
    if !active {
        return Ok((
            false,
            0,
            0.0,
            amount,
            "This promotional coupon is inactive".to_string(),
        ));
    }

    let expires_at: DateTime<Utc> = row.get("expires_at");
    if expires_at < Utc::now() {
        return Ok((
            false,
            0,
            0.0,
            amount,
            "This promotional coupon has expired".to_string(),
        ));
    }

    let max_uses: i32 = row.get("max_uses");
    let current_uses: i32 = row.get("current_uses");
    if current_uses >= max_uses {
        return Ok((
            false,
            0,
            0.0,
            amount,
            "Redemption cap reached for this coupon".to_string(),
        ));
    }

    let discount_percent: i32 = row.get("discount_percent");
    let discount_amount = (amount * (discount_percent as f64) / 100.0).round();
    let final_amount = (amount - discount_amount).max(0.0);

    Ok((
        true,
        discount_percent,
        discount_amount,
        final_amount,
        format!("{}% discount applied successfully", discount_percent),
    ))
}

