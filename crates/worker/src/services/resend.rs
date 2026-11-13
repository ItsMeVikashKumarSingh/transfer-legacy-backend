use crate::config::Config;
use handlebars::Handlebars;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;

#[derive(thiserror::Error, Debug)]
pub enum ResendError {
    #[error("http error: {0}")]
    Http(String),
    #[error("template error: {0}")]
    Template(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Serialize)]
struct ResendPayload<'a> {
    from: &'a str,
    to: Vec<&'a str>,
    subject: String,
    html: String,
}

pub async fn send_email(
    config: &Config,
    template_name: &str,
    to_email: &str,
    params: serde_json::Value,
) -> Result<(), ResendError> {
    let hb = Handlebars::new();

    let html_content = match template_name {
        "admin_created" => include_str!("../../../../docs/templates/admin_created.html"),
        "approver_attestation_request" => include_str!("../../../../docs/templates/approver_attestation_request.html"),
        "beneficiary_claim_available" => include_str!("../../../../docs/templates/beneficiary_claim_available.html"),
        "conflict_hold_notice" => include_str!("../../../../docs/templates/conflict_hold_notice.html"),
        "invite" => include_str!("../../../../docs/templates/invite.html"),
        "owner_reminder_daily" => include_str!("../../../../docs/templates/owner_reminder_daily.html"),
        "owner_reminder_early" => include_str!("../../../../docs/templates/owner_reminder_early.html"),
        "owner_reminder_urgent" => include_str!("../../../../docs/templates/owner_reminder_urgent.html"),
        "password_reset" => include_str!("../../../../docs/templates/password_reset.html"),
        "register_otp" => include_str!("../../../../docs/templates/register_otp.html"),
        "release_ready" => include_str!("../../../../docs/templates/release_ready.html"),
        "security_alert" => include_str!("../../../../docs/templates/security_alert.html"),
        "waitlist_welcome" => include_str!("../../../../docs/templates/waitlist_welcome.html"),
        _ => return Err(ResendError::Template(format!("Unknown template: {}", template_name))),
    };

    // Determine Subject and From address based on template name
    let (subject, from) = match template_name {
        n if n.contains("invite") => (
            "You've been invited to a Digital Inheritance Plan",
            "Transfer Legacy <support@transferlegacy.com>",
        ),
        n if n.contains("password_reset") => (
            "Reset your Transfer Legacy Password",
            "Transfer Legacy <no-reply@transferlegacy.com>",
        ),
        n if n.contains("owner_reminder_early") => (
            "Action Required: Digital Inheritance Heartbeat",
            "Transfer Legacy <no-reply@transferlegacy.com>",
        ),
        n if n.contains("owner_reminder_urgent") => (
            "URGENT: Your Digital Inheritance Plan is Pending",
            "Transfer Legacy <no-reply@transferlegacy.com>",
        ),
        n if n.contains("owner_reminder_daily") => (
            "Daily Reminder: Digital Inheritance Heartbeat",
            "Transfer Legacy <no-reply@transferlegacy.com>",
        ),
        n if n.contains("beneficiary_claim") => (
            "A Digital Inheritance Claim is Available",
            "Transfer Legacy <support@transferlegacy.com>",
        ),
        n if n.contains("approver_attestation") => (
            "Action Required: Witness Attestation Request",
            "Transfer Legacy <support@transferlegacy.com>",
        ),
        n if n.contains("release_ready") => (
            "Action Required: Your Legacy is Ready for Release",
            "Transfer Legacy <support@transferlegacy.com>",
        ),
        n if n.contains("waitlist_welcome") => (
            "Welcome to the Transfer Legacy Waitlist!",
            "Transfer Legacy <waitlist@transferlegacy.com>",
        ),
        _ => (
            "Notification from Transfer Legacy",
            "Transfer Legacy <no-reply@transferlegacy.com>",
        ),
    };

    let params_map: HashMap<String, serde_json::Value> = serde_json::from_value(params)
        .map_err(|e| ResendError::Template(format!("Invalid params: {}", e)))?;

    let rendered_html = hb
        .render_template(&html_content, &params_map)
        .map_err(|e| ResendError::Template(format!("Failed to render template: {}", e)))?;

    // --- Mock Mode Check ---
    if config.resend_api_key.starts_with("re_mock_") {
        println!("✉️ MOCK EMAIL WORKER LOG: To: {}, Subject: {}", to_email, subject);
        return Ok(());
    }

    let client = Client::new();
    let url = "https://api.resend.com/emails";

    let payload = ResendPayload {
        from,
        to: vec![to_email],
        subject: subject.to_string(),
        html: rendered_html,
    };

    let res = client
        .post(url)
        .header("Authorization", format!("Bearer {}", config.resend_api_key))
        .json(&payload)
        .send()
        .await
        .map_err(|e| ResendError::Http(e.to_string()))?;

    if res.status().is_success() {
        Ok(())
    } else {
        let err_text = res.text().await.unwrap_or_default();
        Err(ResendError::Unexpected(err_text))
    }
}
