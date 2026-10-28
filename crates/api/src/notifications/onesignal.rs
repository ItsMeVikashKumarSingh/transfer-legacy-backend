use crate::config::Config;
use reqwest::Client;
use serde::Serialize;
use uuid::Uuid;

#[derive(thiserror::Error, Debug)]
pub enum OneSignalError {
    #[error("http error: {0}")]
    Http(String),
    #[error("unexpected response: {0}")]
    Unexpected(String),
}

#[derive(Debug, Clone, Serialize)]
pub struct PushNotificationPayload {
    pub user_id: Uuid,
    pub title: String,
    pub message: String,
    pub action_url: Option<String>,
    pub data: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct OneSignalAliases {
    external_id: Vec<String>,
}

#[derive(Serialize)]
struct OneSignalBody<'a> {
    app_id: &'a str,
    target_channel: &'a str,
    include_aliases: OneSignalAliases,
    headings: std::collections::HashMap<&'static str, &'a str>,
    contents: std::collections::HashMap<&'static str, &'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    url: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<&'a serde_json::Value>,
}

pub async fn send_push_notification(
    config: &Config,
    payload: &PushNotificationPayload,
) -> Result<bool, OneSignalError> {
    let app_id = match &config.onesignal_app_id {
        Some(id) if !id.is_empty() => id,
        _ => {
            tracing::info!("OneSignal push skipped: ONESIGNAL_APP_ID is not configured");
            return Ok(false);
        }
    };

    let rest_api_key = match &config.onesignal_rest_api_key {
        Some(key) if !key.is_empty() => key,
        _ => {
            tracing::info!("OneSignal push skipped: ONESIGNAL_REST_API_KEY is not configured");
            return Ok(false);
        }
    };

    let mut headings = std::collections::HashMap::new();
    headings.insert("en", payload.title.as_str());

    let mut contents = std::collections::HashMap::new();
    contents.insert("en", payload.message.as_str());

    let body = OneSignalBody {
        app_id,
        target_channel: "push",
        include_aliases: OneSignalAliases {
            external_id: vec![payload.user_id.to_string()],
        },
        headings,
        contents,
        url: payload.action_url.as_deref(),
        data: payload.data.as_ref(),
    };

    let client = Client::new();
    let auth_header = format!("Key {}", rest_api_key);

    let res = client
        .post("https://onesignal.com/api/v1/notifications")
        .header("Authorization", auth_header)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| OneSignalError::Http(e.to_string()))?;

    let status = res.status();
    let body_text = res
        .text()
        .await
        .unwrap_or_else(|_| "<unreadable body>".to_string());

    if !status.is_success() {
        tracing::warn!(
            "OneSignal API returned error status {}: {}",
            status,
            body_text
        );
        return Err(OneSignalError::Unexpected(format!(
            "OneSignal returned {}: {}",
            status, body_text
        )));
    }

    tracing::info!(
        "OneSignal push dispatched successfully for user {}: {}",
        payload.user_id,
        body_text
    );

    Ok(true)
}
