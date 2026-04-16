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
        owner_name: String,
        policy_name: String,
        invite_url: String,
        expires_at: String,
        invite_id: String,
        claim_token: String,
    },
    PasswordReset {
        owner_name: String,
        reset_url: String,
    },
    OwnerReminder {
        urgency: String, // "early", "urgent", "daily"
        owner_name: String,
        policy_name: String,
        grace_deadline: String,
    },
    ClaimAvailable {
        owner_name: String,
        policy_name: String,
    },
    AttestationRequest {
        owner_name: String,
        policy_name: String,
    },
    ConflictHold {
        owner_name: String,
        policy_name: String,
    },
    ReleaseReady {
        owner_name: String,
        policy_name: String,
    },
    SecurityAlert {
        diff_html: String,
        audit_details: String,
    },
}

pub async fn send_notification(
    config: &Config,
    to_email: &str,
    template: NotificationTemplate,
) -> Result<(), BrevoError> {
    let client = Client::new();
    let url = "https://api.brevo.com/v3/smtp/email";

    let platform_blurb = "Transfer Legacy provides secure, non-custodial digital inheritance solutions. Visit transferlegacy.com to learn more.";

    let (template_id_str, extra_params) = match template {
        NotificationTemplate::Invite {
            owner_name,
            policy_name,
            invite_url,
            expires_at,
            invite_id,
            claim_token,
        } => (
            &config.brevo_invite_template_id,
            serde_json::json!({
                "owner_name": owner_name,
                "policy_name": policy_name,
                "invite_url": invite_url,
                "expires_at": expires_at,
                "invite_id": invite_id,
                "claim_token": claim_token,
            }),
        ),
        NotificationTemplate::PasswordReset { owner_name, reset_url } => (
            &config.brevo_password_reset_template_id,
            serde_json::json!({
                "owner_name": owner_name,
                "reset_url": reset_url,
            }),
        ),
        NotificationTemplate::OwnerReminder { urgency, owner_name, policy_name, grace_deadline } => {
            let id = match urgency.as_str() {
                "early" => &config.brevo_owner_reminder_early_template_id,
                "urgent" => &config.brevo_owner_reminder_urgent_template_id,
                _ => &config.brevo_owner_reminder_daily_template_id,
            };
            (id, serde_json::json!({ 
                "owner_name": owner_name,
                "policy_name": policy_name,
                "grace_deadline": grace_deadline,
            }))
        }
        NotificationTemplate::ClaimAvailable { owner_name, policy_name } => (
            &config.brevo_beneficiary_claim_available_template_id,
            serde_json::json!({ 
                "owner_name": owner_name,
                "policy_name": policy_name,
            }),
        ),
        NotificationTemplate::AttestationRequest { owner_name, policy_name } => (
            &config.brevo_approver_attestation_request_template_id,
            serde_json::json!({ 
                "owner_name": owner_name,
                "policy_name": policy_name,
            }),
        ),
        NotificationTemplate::ConflictHold { owner_name, policy_name } => (
            &config.brevo_conflict_hold_notice_template_id,
            serde_json::json!({ 
                "owner_name": owner_name,
                "policy_name": policy_name,
            }),
        ),
        NotificationTemplate::ReleaseReady { owner_name, policy_name } => (
            &config.brevo_release_ready_template_id,
            serde_json::json!({ 
                "owner_name": owner_name,
                "policy_name": policy_name,
            }),
        ),
        NotificationTemplate::SecurityAlert {
            diff_html,
            audit_details,
        } => (
            &config.security_template_id,
            serde_json::json!({
                "diff_html": diff_html,
                "audit_details": audit_details,
            }),
        ),
    };

    let template_id: i64 = template_id_str
        .parse()
        .map_err(|_| BrevoError::Unexpected("Invalid template ID".into()))?;

    // Centralized parameters for all emails
    let mut params = serde_json::json!({
        "brand_name": config.brand_name,
        "app_url": config.app_url,
        "platform_description": platform_blurb,
    });

    // Merge extra params
    if let Value::Object(ref mut map) = params {
        if let Value::Object(extra_map) = extra_params {
            for (k, v) in extra_map {
                map.insert(k, v);
            }
        }
    }

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
            owner_name: "A Policy Owner".to_string(),
            policy_name: "Legacy Plan".to_string(),
            invite_url: format!("{}/invite/claim?invite_id={}&token={}", config.app_url, invite_id, claim_token),
            invite_id: invite_id.to_string(),
            claim_token: claim_token.to_string(),
            expires_at: expires_at.to_string(),
        },
    )
    .await
}
