use crate::notifications::resend::NotificationTemplate;
use crate::tests::test_utils::spawn_app;
use std::sync::Arc;
use uuid::Uuid;

#[tokio::test]
#[ignore]
async fn test_full_stack_notification_flow() {
    // 1. Initialize the app with real local config (.env.local)
    // This will load the RESEND_API_KEY we just added.
    let ctx = spawn_app().await;

    let owner_email = "vikashbro111@gmail.com";
    let beneficiary_email = "ak8826778@gmail.com";
    let owner_id = Uuid::new_v4();

    println!(
        "DEBUG: Using RESEND_API_KEY: {}...",
        &ctx.config.resend_api_key[..7]
    );

    println!("--- Testing Password Reset Notification (Resend Template: password_reset) ---");
    let template = NotificationTemplate::PasswordReset {
        owner_name: "Vikash Kumar Singh".to_string(),
        reset_url: "https://transferlegacy.com/auth/reset?token=test-token".to_string(),
    };

    // Test direct notification via AppState (this verifies OpenBao config + Resend service)
    ctx.state
        .notify(owner_id, owner_email, template)
        .await
        .expect("Failed to send real email via Resend");
    println!(
        "SUCCESS: REAL E-MAIL SENT to {}. Please check inbox for local template branding.",
        owner_email
    );

    println!("--- Testing Invitation Notification (Resend Template: invite) ---");
    let invite_template = NotificationTemplate::Invite {
        owner_name: "Vikash Kumar Singh".to_string(),
        policy_name: "Family Trust".to_string(),
        invite_url: "https://transferlegacy.com/invite/claim?id=test".to_string(),
        invite_id: "test-invite-123".to_string(),
        claim_token: "token-abc".to_string(),
        expires_at: "2026-12-31".to_string(),
    };

    ctx.state
        .notify(owner_id, beneficiary_email, invite_template)
        .await
        .expect("Failed to send real invitation email");
    println!("SUCCESS: REAL INVITATION SENT to {}.", beneficiary_email);

    println!("--- Live Verification Complete ---");
}
