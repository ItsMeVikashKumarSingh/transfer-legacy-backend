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

    // Load template from local docs/templates
    let template_path = format!("docs/templates/{}.html", template_name);

    let html_content = std::fs::read_to_string(&template_path).map_err(|e| {
        ResendError::Template(format!("Failed to read template {}: {}", template_name, e))
    })?;

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
