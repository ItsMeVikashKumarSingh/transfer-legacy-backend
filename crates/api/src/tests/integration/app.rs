use crate::tests::test_utils::spawn_app;
use transfer_legacy_shared_types::models::app::{BrandingConfig, WaitlistSignupRequest};
use serde_json::json;

#[tokio::test]
async fn test_public_branding_retrieval() {
    let ctx = spawn_app().await;

    // Fetch branding
    let res = ctx.client.get("/v1/app/branding").await;
    res.assert_status_success();
    
    let envelope: crate::errors::SuccessEnvelope<BrandingConfig> = res.json();
    assert_eq!(envelope.data.brand_name, "Transfer Legacy");
}

#[tokio::test]
async fn test_public_waitlist_signup() {
    let ctx = spawn_app().await;
    
    let signup_req = WaitlistSignupRequest {
        email: "waitlist@example.com".to_string(),
        name: Some("Waitlist User".to_string()),
        metadata: Some(json!({"utm_source": "unit_test"})),
    };

    let res = ctx.client.post("/v1/app/waitlist").json(&signup_req).await;
    res.assert_status_success();

    // Verify DB insertion
    let row = sqlx::query(
        "SELECT email, name FROM app.waitlist WHERE email = $1"
    )
    .bind("waitlist@example.com")
    .fetch_one(&ctx.db)
    .await
    .expect("Email not found in waitlist table");

    use sqlx::Row;
    let email: String = row.get("email");
    let name: Option<String> = row.get("name");

    assert_eq!(email, "waitlist@example.com");
    assert_eq!(name, Some("Waitlist User".into()));
}

#[tokio::test]
async fn test_admin_routes_unauthorized() {
    let ctx = spawn_app().await;

    // Try to list waitlist without token
    let res = ctx.client.get("/v1/ops/waitlist").await;
    assert_eq!(res.status_code(), axum::http::StatusCode::UNAUTHORIZED);

    // Try to update branding without token
    let res = ctx.client.put("/v1/ops/branding").json(&json!({})).await;
    assert_eq!(res.status_code(), axum::http::StatusCode::UNAUTHORIZED);
}
