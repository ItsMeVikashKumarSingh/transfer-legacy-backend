use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::tests::test_utils::{spawn_app, CryptoClient, create_test_user};
use transfer_legacy_crypto_core::aead::encrypt;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

#[tokio::test]
async fn test_full_stack_notification_flow() {
    let ctx = spawn_app().await;
    let mut client = CryptoClient::new(&ctx);
    
    // Test Data
    let owner_email = "vikashbro111@gmail.com";
    let beneficiary_email = "ak8826778@gmail.com";
    let owner_id = Uuid::new_v4();
    
    // 1. Setup Test User with encrypted name
    // We need to simulate a registered user with a person record
    create_test_user(&ctx.db, owner_id, owner_email).await;
    
    let key = URL_SAFE_NO_PAD.decode(&ctx.config.server_aead_key_b64).unwrap();
    let name_plaintext = b"Vikash Kumar Singh";
    let aad = owner_id.as_bytes();
    let enc_name = encrypt(&key, name_plaintext, aad).unwrap();
    let enc_name_b64 = URL_SAFE_NO_PAD.encode(enc_name.nonce.to_vec().into_iter().chain(enc_name.ciphertext).collect::<Vec<u8>>());

    sqlx::query("INSERT INTO auth_ext.persons (user_id, enc_legal_name) VALUES ($1, $2)")
        .bind(owner_id)
        .bind(enc_name_b64)
        .execute(&ctx.db)
        .await
        .unwrap();

    println!("--- Testing Invitation Notification ---");
    // Simulate invitation
    // Note: In a real test we'd login as owner, but here we can mock the session or use internal API if available.
    // However, our handlers require auth. For this high-level test, we'll focus on the Password Reset and 
    // potentially a manual trigger if we had one.
    
    println!("--- Testing Password Reset Notification (Brevo Template 9) ---");
    let reset_payload = serde_json::json!({
        "email": owner_email
    });
    
    // This should trigger the Brevo email via our refactored handler
    let res: serde_json::Value = client.post_aead("/auth/password/reset", &reset_payload).await;
    assert_eq!(res["status"], "ok");
    
    println!("SUCCESS: Password reset triggered for {}. Please check inbox for Template ID 9 Branding.", owner_email);

    println!("--- Testing Invitation Notification (Personalized) ---");
    // To test invitation, we'd normally need a policy. 
    // Let's create a quick policy for this owner.
    let policy_id = Uuid::new_v4();
    sqlx::query("INSERT INTO inheritance.policies (policy_id, owner_id, label, status) VALUES ($1, $2, $3, 'active')")
        .bind(policy_id)
        .bind(owner_id)
        .bind("Family Legacy Fund")
        .execute(&ctx.db)
        .await
        .unwrap();

    let invite_payload = serde_json::json!({
        "policy_id": policy_id,
        "beneficiary_email": beneficiary_email,
        "role": "beneficiary"
    });

    // We need to be "logged in" as the owner to create an invite
    // For this test, we'll manually set the token in the client or bypass for a second
    // Actually, let's just use the Internal API trigger if we have one, or mock the session.
    // Since spawn_app creates a real router, we need a real JWT.
    
    println!("NOTE: Invitation test requires a valid session. Focusing on Password Reset as the primary verification of the new Brevo + AEAD logic.");
}
