use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum SupabaseError {
    #[error("http error")]
    Http,
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
        .header("Authorization", format!("Bearer {}", config.supabase_publishable_key))
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
    let url = format!("{}/auth/v1/token?grant_type=refresh_token", config.supabase_url);
    let res = client
        .post(url)
        .header("apikey", config.supabase_publishable_key.as_str())
        .header("Authorization", format!("Bearer {}", config.supabase_publishable_key))
        .json(&RefreshRequest { refresh_token })
        .send()
        .await
        .map_err(|_| SupabaseError::Http)?;

    if res.status().is_success() {
        res.json::<RefreshResponse>().await.map_err(|_| SupabaseError::Unexpected)
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
) -> Result<(), SupabaseError> {
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
        Ok(())
    } else {
        Err(SupabaseError::Unexpected)
    }
}
