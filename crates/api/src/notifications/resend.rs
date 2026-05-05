use crate::config::Config;
use handlebars::Handlebars;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
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
    subject: &'a str,
    html: String,
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
    AdminCreated {
        email: String,
        password: String,
    },
    WaitlistWelcome {
        owner_name: String,
        position: i64,
    },
}

impl NotificationTemplate {
    fn subject(&self) -> &str {
        match self {
            Self::Invite { .. } => "You've been invited to a Digital Inheritance Plan",
            Self::PasswordReset { .. } => "Reset your Transfer Legacy Password",
            Self::OwnerReminder { urgency, .. } => match urgency.as_str() {
                "early" => "Action Required: Digital Inheritance Heartbeat",
                "urgent" => "URGENT: Your Digital Inheritance Plan is Pending",
                _ => "Daily Reminder: Digital Inheritance Heartbeat",
            },
            Self::ClaimAvailable { .. } => "A Digital Inheritance Claim is Available",
            Self::AttestationRequest { .. } => "Action Required: Witness Attestation Request",
            Self::ConflictHold { .. } => "Security Notice: Inheritance Conflict Hold",
            Self::ReleaseReady { .. } => "Action Required: Your Legacy is Ready for Release",
            Self::SecurityAlert { .. } => "Security Alert: Platform Configuration Changed",
            Self::AdminCreated { .. } => "Administrator Access Granted",
            Self::WaitlistWelcome { .. } => "Welcome to the Transfer Legacy Waitlist!",
        }
    }

    fn template_name(&self) -> String {
        match self {
            Self::Invite { .. } => "invite".into(),
            Self::PasswordReset { .. } => "password_reset".into(),
            Self::OwnerReminder { urgency, .. } => format!("owner_reminder_{}", urgency),
            Self::ClaimAvailable { .. } => "beneficiary_claim_available".into(),
            Self::AttestationRequest { .. } => "approver_attestation_request".into(),
            Self::ConflictHold { .. } => "conflict_hold_notice".into(),
            Self::ReleaseReady { .. } => "release_ready".into(),
            Self::SecurityAlert { .. } => "security_alert".into(),
            Self::AdminCreated { .. } => "admin_created".into(),
            Self::WaitlistWelcome { .. } => "waitlist_welcome".into(),
        }
    }

    fn from_address(&self) -> &str {
        match self {
            Self::Invite { .. }
            | Self::ClaimAvailable { .. }
            | Self::AttestationRequest { .. }
            | Self::ConflictHold { .. }
            | Self::ReleaseReady { .. } => "Transfer Legacy <support@transferlegacy.com>",
            | Self::SecurityAlert { .. } => "Transfer Legacy <security@transferlegacy.com>",
            Self::AdminCreated { .. } => "Transfer Legacy Control <security@transferlegacy.com>",
            Self::WaitlistWelcome { .. } => "Transfer Legacy <waitlist@transferlegacy.com>",
        }
    }
}

pub async fn send_notification(
    config: &Config,
    to_email: &str,
    template: NotificationTemplate,
) -> Result<(), ResendError> {
    let mut hb = Handlebars::new();

    // Load template from local docs/templates or parent directory (for workspace tests)
    let template_name = template.template_name();
    let paths = vec![
        format!("docs/templates/{}.html", template_name),
        format!("../../docs/templates/{}.html", template_name),
    ];

    let mut html_content = None;
    for path in &paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            html_content = Some(content);
            break;
        }
    }

    let html_content = html_content.ok_or_else(|| {
        ResendError::Template(format!(
            "Failed to find template {} in paths: {:?}",
            template_name, paths
        ))
    })?;

    // Centralized parameters for all emails
    let mut params = HashMap::new();
    params.insert("brand_name", config.brand_name.clone());
    params.insert("app_url", config.app_url.clone());
    params.insert(
        "platform_description",
        "Transfer Legacy provides secure, non-custodial digital inheritance solutions.".to_string(),
    );

    // Map template-specific params
    match &template {
        NotificationTemplate::Invite {
            owner_name,
            policy_name,
            invite_url,
            expires_at,
            ..
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
            params.insert("invite_url", invite_url.clone());
            params.insert("expires_at", expires_at.clone());
        }
        NotificationTemplate::PasswordReset {
            owner_name,
            reset_url,
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("reset_url", reset_url.clone());
        }
        NotificationTemplate::OwnerReminder {
            owner_name,
            policy_name,
            grace_deadline,
            ..
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
            params.insert("grace_deadline", grace_deadline.clone());
        }
        NotificationTemplate::ClaimAvailable {
            owner_name,
            policy_name,
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
        }
        NotificationTemplate::AttestationRequest {
            owner_name,
            policy_name,
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
        }
        NotificationTemplate::ConflictHold {
            owner_name,
            policy_name,
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
        }
        NotificationTemplate::ReleaseReady {
            owner_name,
            policy_name,
        } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("policy_name", policy_name.clone());
        }
        NotificationTemplate::SecurityAlert {
            diff_html,
            audit_details,
        } => {
            params.insert("diff_html", diff_html.clone());
            params.insert("audit_details", audit_details.clone());
        }
        NotificationTemplate::AdminCreated { email, password } => {
            params.insert("admin_email", email.clone());
            params.insert("admin_password", password.clone());
        }
        NotificationTemplate::WaitlistWelcome { owner_name, position } => {
            params.insert("owner_name", owner_name.clone());
            params.insert("position", position.to_string());
        }
    }

    let mut root = HashMap::new();
    root.insert("params", params);

    let rendered_html = hb
        .render_template(&html_content, &root)
        .map_err(|e| ResendError::Template(format!("Failed to render template: {}", e)))?;

    let client = Client::new();
    let url = "https://api.resend.com/emails";

    let payload = ResendPayload {
        from: template.from_address(),
        to: vec![to_email],
        subject: template.subject(),
        html: rendered_html,
    };

    // --- Mock Mode Check ---
    if config.resend_api_key.starts_with("re_mock_") {
        println!("✉️ MOCK EMAIL LOG: To: {}, Subject: {}", to_email, template.subject());
        return Ok(());
    }

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
