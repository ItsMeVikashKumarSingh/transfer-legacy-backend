use crate::tests::test_utils::spawn_app;
use transfer_legacy_shared_types::models::ops::{OpsLoginRequest, OpsLoginResponse};
use uuid::Uuid;

#[tokio::test]
async fn test_ops_fallback_login() {
    // Set fallback credentials in environment before spawning
    std::env::set_var("OPS_ADMIN_EMAIL", "admin@transferlegacy.com");
    std::env::set_var("OPS_ADMIN_PASSWORD", "Admin@123");
    
    let ctx = spawn_app().await;
    
    let login_req = OpsLoginRequest {
        email: "admin@transferlegacy.com".to_string(),
        password: "Admin@123".to_string(),
    };
    
    let login_res = ctx.client
        .post("/v1/ops/login")
        .json(&login_req)
        .await;
        
    login_res.assert_status_success();
    
    let login_data: OpsLoginResponse = login_res.json();
    assert!(!login_data.token.is_empty());
    assert_eq!(login_data.admin.email, "admin@transferlegacy.com");
    assert_eq!(login_data.admin.id, Uuid::nil());
    assert_eq!(login_data.admin.role_name, "super_admin");
}

#[tokio::test]
async fn test_ops_login_invalid_credentials() {
    let ctx = spawn_app().await;
    
    let login_req = OpsLoginRequest {
        email: "wrong@example.com".to_string(),
        password: "WrongPassword".to_string(),
    };
    
    let res = ctx.client
        .post("/v1/ops/login")
        .json(&login_req)
        .await;
        
    res.assert_status_unauthorized();
}
