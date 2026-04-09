use chrono::{DateTime, Utc};
use metrics::histogram;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};
use std::time::Instant;
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
