use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseError {
    #[error("http error")]
    Http,
    #[error("user already exists")]
    UserAlreadyExists,
    #[error("unexpected response")]
    Unexpected,
}

#[derive(Debug, Serialize)]
struct RecoverRequest<'a> {
    email: &'a str,
}

#[derive(Debug, Serialize)]
struct RefreshRequest<'a> {
    refresh_token: &'a str,
}

#[derive(Debug, Deserialize)]
pub struct RefreshResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: i64,
}

pub async fn send_password_recovery(config: &Config, email: &str) -> Result<(), SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/recover", config.supabase_url);
    let res = client
        .post(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_publishable_key),
        )
        .json(&RecoverRequest { email })
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(SupabaseError::Unexpected)
    }
}

pub async fn refresh_session(
    config: &Config,
    refresh_token: &str,
) -> Result<RefreshResponse, SupabaseError> {
    let client = Client::new();
    let url = format!(
        "{}/auth/v1/token?grant_type=refresh_token",
        config.supabase_url
    );
    let res = client
        .post(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_publishable_key),
        )
        .json(&RefreshRequest { refresh_token })
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        res.json::<RefreshResponse>()
            .await
            .map_err(|_| SupabaseError::Unexpected)
    } else {
        Err(SupabaseError::Unexpected)
    }
}

pub async fn logout_session(config: &Config, access_token: &str) -> Result<(), SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/logout", config.supabase_url);
    let res = client
        .post(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        Ok(())
    } else {
        Err(SupabaseError::Unexpected)
    }
}

pub async fn reset_password_with_token(
    config: &Config,
    access_token: &str,
    new_password: &str,
) -> Result<uuid::Uuid, SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/user", config.supabase_url);
    let res = client
        .put(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header("Authorization", format!("Bearer {}", access_token))
        .json(&serde_json::json!({ "password": new_password }))
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        let body: serde_json::Value = res.json().await.map_err(|_| SupabaseError::Unexpected)?;
        let id_str = body.get("id").and_then(|v| v.as_str()).ok_or(SupabaseError::Unexpected)?;
        let user_id = uuid::Uuid::parse_str(id_str).map_err(|_| SupabaseError::Unexpected)?;
        Ok(user_id)
    } else {
        Err(SupabaseError::Unexpected)
    }
}

pub async fn generate_recovery_link(config: &Config, email: &str) -> Result<String, SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/admin/generate_link", config.supabase_url);
    let res = client
        .post(url)
        .header("apikey", config.supabase_secret_key.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_secret_key),
        )
        .json(&serde_json::json!({
            "type": "recovery",
            "email": email
        }))
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        let body: serde_json::Value = res.json().await.map_err(|_| SupabaseError::Unexpected)?;
        let link = body
            .get("action_link")
            .and_then(|v| v.as_str())
            .ok_or(SupabaseError::Unexpected)?;
        Ok(link.to_string())
    } else {
        Err(SupabaseError::Unexpected)
    }
}

pub async fn register_user_in_supabase(
    config: &Config,
    user_id: uuid::Uuid,
    email: &str,
) -> Result<(), SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/admin/users", config.supabase_url);
    let res = client
        .post(url)
        .header("apikey", config.supabase_secret_key.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_secret_key),
        )
        .json(&serde_json::json!({
            "id": user_id.to_string(),
            "email": email,
            "email_confirm": true
        }))
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    let status = res.status();
    if status.is_success() || status == reqwest::StatusCode::CONFLICT || status == reqwest::StatusCode::UNPROCESSABLE_ENTITY {
        Ok(())
    } else {
        let body = res.text().await.unwrap_or_default();
        if body.contains("23505") || body.contains("duplicate key") || body.contains("already exists") {
            Err(SupabaseError::UserAlreadyExists)
        } else {
            tracing::error!("Supabase Admin user creation failed: status={}, body={}", status, body);
            Err(SupabaseError::Unexpected)
        }
    }
}

pub async fn delete_user_in_supabase(
    config: &Config,
    user_id: uuid::Uuid,
) -> Result<(), SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/admin/users/{}", config.supabase_url, user_id);
    let res = client
        .delete(url)
        .header("apikey", config.supabase_secret_key.as_str())
        .header(
            "Authorization",
            format!("Bearer {}", config.supabase_secret_key),
        )
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    let status = res.status();
    if status.is_success() || status == reqwest::StatusCode::NOT_FOUND {
        Ok(())
    } else {
        let err_body = res.text().await.unwrap_or_default();
        tracing::error!("Supabase Admin user deletion failed: status={}, body={}", status, err_body);
        Err(SupabaseError::Unexpected)
    }
}

pub async fn get_user_id_from_token(config: &Config, access_token: &str) -> Result<uuid::Uuid, SupabaseError> {
    let client = Client::new();
    let url = format!("{}/auth/v1/user", config.supabase_url);
    let res = client
        .get(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        let body: serde_json::Value = res.json().await.map_err(|_| SupabaseError::Unexpected)?;
        let id_str = body.get("id").and_then(|v| v.as_str()).ok_or(SupabaseError::Unexpected)?;
        let user_id = uuid::Uuid::parse_str(id_str).map_err(|_| SupabaseError::Unexpected)?;
        Ok(user_id)
    } else {
        Err(SupabaseError::Unexpected)
    }
}

