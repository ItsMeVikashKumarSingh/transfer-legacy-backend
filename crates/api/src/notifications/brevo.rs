use reqwest::Client;
use serde::Serialize;
use serde_json::Value;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum BrevoError {
    #[error("http error")]
    Http,
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Serialize)]
struct EmailAddress<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct SendTemplateRequest<'a> {
    to: Vec<EmailAddress<'a>>,
    #[serde(rename = "templateId")]
    template_id: i64,
    params: Value,
}

pub enum NotificationTemplate {
    Invite {
        invite_id: String,
        claim_token: String,
        expires_at: String,
    },
    PasswordReset {
        reset_link: String,
    },
    OwnerReminder {
        urgency: String, // "early", "urgent", "daily"
        deadline: String,
    },
    ClaimAvailable {
        policy_id: String,
    },
}

pub async fn send_notification(
    config: &Config,
    to_email: &str,
    template: NotificationTemplate,
) -> Result<(), BrevoError> {
    let client = Client::new();
    let url = "https://api.brevo.com/v3/smtp/email";

    let (template_id_str, params) = match template {
        NotificationTemplate::Invite { invite_id, claim_token, expires_at } => (
            &config.brevo_invite_template_id,
            serde_json::json!({
                "brand_name": config.brand_name,
                "app_url": config.app_url,
                "invite_id": invite_id,
                "claim_token": claim_token,
                "expires_at": expires_at,
            }),
        ),
        NotificationTemplate::PasswordReset { reset_link } => (
            &config.brevo_invite_template_id, // Defaulting for now if no specific ID yet
            serde_json::json!({
                "brand_name": config.brand_name,
                "reset_link": reset_link,
            }),
        ),
        NotificationTemplate::OwnerReminder { urgency, deadline } => {
            let id = match urgency.as_str() {
                "early" => &config.brevo_owner_reminder_early_template_id,
                "urgent" => &config.brevo_owner_reminder_urgent_template_id,
                _ => &config.brevo_owner_reminder_daily_template_id,
            };
            (id, serde_json::json!({ "deadline": deadline }))
        }
        NotificationTemplate::ClaimAvailable { policy_id } => (
            &config.brevo_beneficiary_claim_available_template_id,
            serde_json::json!({ "policy_id": policy_id }),
        ),
    };

    let template_id: i64 = template_id_str.parse().map_err(|_| BrevoError::Unexpected("Invalid template ID".into()))?;

    let payload = SendTemplateRequest {
        to: vec![EmailAddress { email: to_email }],
        template_id,
        params,
    };

    let res = client
        .post(url)
        .header("api-key", config.brevo_api_key.as_str())
        .json(&payload)
        .send()
        .await
        .map_err(|_| BrevoError::Http)?;

    if res.status().is_success() {
        Ok(())
    } else {
        let err_text = res.text().await.unwrap_or_default();
        Err(BrevoError::Unexpected(err_text))
    }
}

// Legacy wrapper to maintain compatibility while refactoring
pub async fn send_invite_email(
    config: &Config,
    email: &str,
    invite_id: &str,
    claim_token: &str,
    expires_at: &str,
) -> Result<(), BrevoError> {
    send_notification(
        config,
        email,
        NotificationTemplate::Invite {
            invite_id: invite_id.to_string(),
            claim_token: claim_token.to_string(),
            expires_at: expires_at.to_string(),
        },
    )
    .await
}
