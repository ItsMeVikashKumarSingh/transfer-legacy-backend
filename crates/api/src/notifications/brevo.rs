use reqwest::Client;
use serde::Serialize;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum BrevoError {
    #[error("http error")]
    Http,
    #[error("unexpected response")]
    Unexpected,
}

#[derive(Serialize)]
struct EmailAddress<'a> {
    email: &'a str,
}

#[derive(Serialize)]
struct SendTemplateRequest<'a> {
    to: Vec<EmailAddress<'a>>,
    templateId: i64,
    params: serde_json::Value,
}

pub async fn send_invite_email(
    config: &Config,
    email: &str,
    invite_id: &str,
    claim_token: &str,
    expires_at: &str,
) -> Result<(), BrevoError> {
    let client = Client::new();
    let url = "https://api.brevo.com/v3/smtp/email";

    let template_id: i64 = config
        .brevo_invite_template_id
        .parse()
        .map_err(|_| BrevoError::Unexpected)?;

    let payload = SendTemplateRequest {
        to: vec![EmailAddress { email }],
        templateId: template_id,
        params: serde_json::json!({
            "brand_name": config.brand_name,
            "app_url": config.app_url,
            "invite_id": invite_id,
            "claim_token": claim_token,
            "expires_at": expires_at,
        }),
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
        Err(BrevoError::Unexpected)
    }
}
