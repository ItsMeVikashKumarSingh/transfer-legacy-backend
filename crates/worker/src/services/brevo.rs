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
    #[serde(rename = "templateId")]
    template_id: i64,
    params: serde_json::Value,
}

pub async fn send_template_email(
    config: &Config,
    template_id: &str,
    email: &str,
    params: serde_json::Value,
) -> Result<(), BrevoError> {
    let client = Client::new();
    let url = "https://api.brevo.com/v3/smtp/email";
    let template_id: i64 = template_id.parse().map_err(|_| BrevoError::Unexpected)?;

    let payload = SendTemplateRequest {
        to: vec![EmailAddress { email }],
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
        Err(BrevoError::Unexpected)
    }
}
