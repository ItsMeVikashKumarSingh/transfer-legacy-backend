use axum::http::HeaderValue;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub port: u16,
    pub allowed_origins: Vec<HeaderValue>,
    pub app_url: String,
    pub brand_name: String,
    pub internal_api_token: Option<String>,
    pub database_url: String,
    pub redis_url: String,
    pub openbao_addr: String,
    pub openbao_token: String,
    pub openbao_version: u32,
    pub b2_key_id: String,
    pub b2_app_key: String,
    pub b2_bucket_name: String,
    pub b2_audit_bucket_name: String,
    pub b2_backup_bucket_name: String,
    pub b2_endpoint_url: String,
    pub server_aead_key_b64: String,
    pub opaque_server_setup_b64: String,
    pub jwt_secret: String,
    pub supabase_url: String,
    pub supabase_publishable_key: String,
    pub supabase_secret_key: String,
    pub server_hmac_secret: String,
    pub brevo_api_key: String,
    pub owner_email: String,
    pub security_template_id: String,
    pub brevo_invite_template_id: String,
    pub brevo_owner_reminder_early_template_id: String,
    pub brevo_owner_reminder_urgent_template_id: String,
    pub brevo_owner_reminder_daily_template_id: String,
    pub brevo_beneficiary_claim_available_template_id: String,
    pub brevo_approver_attestation_request_template_id: String,
    pub brevo_conflict_hold_notice_template_id: String,
    pub brevo_release_ready_template_id: String,
    pub brevo_password_reset_template_id: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing required env var: {0}")]
    MissingVar(&'static str),
    #[error("invalid port in TL_PORT: {0}")]
    InvalidPort(String),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected openbao response")]
    OpenBaoResponse,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OpenBaoSecrets {
    pub server_aead_key: String,
    pub opaque_server_setup: String,
    pub jwt_secret: String,
    pub server_hmac_secret: String,
    pub brevo_api_key: String,
    pub supabase_url: String,
    pub supabase_publishable_key: String,
    pub supabase_secret_key: String,
    pub backblaze_b2_key_id: String,
    pub backblaze_b2_app_key: String,
    pub backblaze_b2_bucket_name: String,
    pub backblaze_b2_audit_bucket_name: String,
    pub backblaze_b2_backup_bucket_name: String,
    pub backblaze_b2_endpoint_url: String,
    pub internal_api_token: Option<String>,
    pub owner_email: String,
    pub security_template_id: Option<String>,
    pub app_url: String,
    pub brand_name: String,
    pub redis_url: Option<String>,
    pub database_url: Option<String>,
    pub brevo_invite_template_id: String,
    pub brevo_owner_reminder_early_template_id: String,
    pub brevo_owner_reminder_urgent_template_id: String,
    pub brevo_owner_reminder_daily_template_id: String,
    pub brevo_beneficiary_claim_available_template_id: String,
    pub brevo_approver_attestation_request_template_id: String,
    pub brevo_conflict_hold_notice_template_id: String,
    pub brevo_release_ready_template_id: String,
    pub brevo_password_reset_template_id: Option<String>,
}

impl Config {
    pub async fn load() -> Result<Self, ConfigError> {
        // ... (existing OpenBao load logic)
        // 1. Gather Bootstrap Env Vars
        let bind_addr = env::var("TL_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port_str = env::var("TL_PORT").unwrap_or_else(|_| "8080".to_string());
        let port = port_str
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort(port_str))?;

        let openbao_addr =
            env::var("OPENBAO_ADDR").map_err(|_| ConfigError::MissingVar("OPENBAO_ADDR"))?;
        let role_id = env::var("ROLE_ID").map_err(|_| ConfigError::MissingVar("ROLE_ID"))?;
        let secret_id = env::var("SECRET_ID").map_err(|_| ConfigError::MissingVar("SECRET_ID"))?;

        // 3. Authenticate with AppRole to get a token
        let client = Client::new();
        let auth_url = format!("{}/v1/auth/approle/login", openbao_addr);
        let auth_res = client
            .post(auth_url)
            .json(&serde_json::json!({
                "role_id": role_id,
                "secret_id": secret_id
            }))
            .send()
            .await?;

        if !auth_res.status().is_success() {
            return Err(ConfigError::OpenBaoResponse);
        }

        #[derive(Deserialize)]
        struct AuthResponse {
            auth: AuthData,
        }
        #[derive(Deserialize)]
        struct AuthData {
            client_token: String,
        }

        let auth_body: AuthResponse = auth_res.json().await?;
        let openbao_token = auth_body.auth.client_token;

        // 3. Fetch All Configuration from OpenBao
        let kv_url = format!("{}/v1/secret/data/transfer-legacy/prod", openbao_addr);
        let res = client
            .get(kv_url)
            .header("X-Vault-Token", &openbao_token)
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(ConfigError::OpenBaoResponse);
        }

        #[derive(Deserialize)]
        struct VaultResponse {
            data: VaultData,
        }
        #[derive(Deserialize)]
        struct VaultData {
            data: OpenBaoSecrets,
            metadata: VaultMetadata,
        }
        #[derive(Deserialize)]
        struct VaultMetadata {
            version: u32,
        }

        let body: VaultResponse = res.json().await?;
        let s = body.data.data;
        let openbao_version = body.data.metadata.version;

        let allowed_origins = vec![HeaderValue::from_str(&s.app_url).unwrap()];

        Ok(Self {
            bind_addr,
            port,
            allowed_origins,
            app_url: s.app_url,
            brand_name: s.brand_name,
            internal_api_token: s.internal_api_token,
            database_url: s.database_url.unwrap_or_default(),
            redis_url: s.redis_url.unwrap_or_default(),
            openbao_addr,
            openbao_token,
            openbao_version,
            b2_key_id: s.backblaze_b2_key_id,
            b2_app_key: s.backblaze_b2_app_key,
            b2_bucket_name: s.backblaze_b2_bucket_name,
            b2_audit_bucket_name: s.backblaze_b2_audit_bucket_name,
            b2_backup_bucket_name: s.backblaze_b2_backup_bucket_name,
            b2_endpoint_url: s.backblaze_b2_endpoint_url,
            server_aead_key_b64: s.server_aead_key,
            opaque_server_setup_b64: s.opaque_server_setup,
            jwt_secret: s.jwt_secret,
            supabase_url: s.supabase_url,
            supabase_publishable_key: s.supabase_publishable_key,
            supabase_secret_key: s.supabase_secret_key,
            server_hmac_secret: s.server_hmac_secret,
            brevo_api_key: s.brevo_api_key,
            owner_email: s.owner_email,
            security_template_id: s.security_template_id.unwrap_or_else(|| "9".to_string()),
            brevo_invite_template_id: s.brevo_invite_template_id,
            brevo_owner_reminder_early_template_id: s.brevo_owner_reminder_early_template_id,
            brevo_owner_reminder_urgent_template_id: s.brevo_owner_reminder_urgent_template_id,
            brevo_owner_reminder_daily_template_id: s.brevo_owner_reminder_daily_template_id,
            brevo_beneficiary_claim_available_template_id: s.brevo_beneficiary_claim_available_template_id,
            brevo_approver_attestation_request_template_id: s.brevo_approver_attestation_request_template_id,
            brevo_conflict_hold_notice_template_id: s.brevo_conflict_hold_notice_template_id,
            brevo_release_ready_template_id: s.brevo_release_ready_template_id,
            brevo_password_reset_template_id: s.brevo_password_reset_template_id.unwrap_or_else(|| "9".to_string()),
        })
    }

    pub fn from_env() -> Result<Self, ConfigError> {
        dotenvy::from_filename(".env.local").ok();
        dotenvy::dotenv().ok();

        let bind_addr = env::var("TL_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port_str = env::var("TL_PORT").unwrap_or_else(|_| "8080".to_string());
        let port = port_str
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort(port_str))?;

        let app_url = env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());
        let allowed_origins = vec![HeaderValue::from_str(&app_url).unwrap()];

        Ok(Self {
            bind_addr,
            port,
            allowed_origins,
            app_url,
            brand_name: env::var("BRAND_NAME").unwrap_or_else(|_| "Transfer Legacy".into()),
            internal_api_token: env::var("INTERNAL_API_TOKEN").ok(),
            database_url: env::var("DATABASE_URL").map_err(|_| ConfigError::MissingVar("DATABASE_URL"))?,
            redis_url: env::var("REDIS_URL").map_err(|_| ConfigError::MissingVar("REDIS_URL"))?,
            openbao_addr: env::var("OPENBAO_ADDR").unwrap_or_default(),
            openbao_token: "".to_string(),
            openbao_version: 0,
            b2_key_id: env::var("B2_KEY_ID").unwrap_or_default(),
            b2_app_key: env::var("B2_APP_KEY").unwrap_or_default(),
            b2_bucket_name: env::var("B2_BUCKET_NAME").unwrap_or_default(),
            b2_audit_bucket_name: env::var("B2_AUDIT_BUCKET_NAME").unwrap_or_default(),
            b2_backup_bucket_name: env::var("B2_BACKUP_BUCKET_NAME").unwrap_or_default(),
            b2_endpoint_url: env::var("B2_ENDPOINT_URL").unwrap_or_default(),
            server_aead_key_b64: env::var("SERVER_AEAD_KEY_B64").map_err(|_| ConfigError::MissingVar("SERVER_AEAD_KEY_B64"))?,
            opaque_server_setup_b64: env::var("OPAQUE_SERVER_SETUP_B64").map_err(|_| ConfigError::MissingVar("OPAQUE_SERVER_SETUP_B64"))?,
            jwt_secret: env::var("JWT_SECRET").map_err(|_| ConfigError::MissingVar("JWT_SECRET"))?,
            supabase_url: env::var("SUPABASE_URL").map_err(|_| ConfigError::MissingVar("SUPABASE_URL"))?,
            supabase_publishable_key: env::var("SUPABASE_PUBLISHABLE_KEY").map_err(|_| ConfigError::MissingVar("SUPABASE_PUBLISHABLE_KEY"))?,
            supabase_secret_key: env::var("SUPABASE_SECRET_KEY").map_err(|_| ConfigError::MissingVar("SUPABASE_SECRET_KEY"))?,
            server_hmac_secret: env::var("SERVER_HMAC_SECRET").map_err(|_| ConfigError::MissingVar("SERVER_HMAC_SECRET"))?,
            brevo_api_key: env::var("BREVO_API_KEY").map_err(|_| ConfigError::MissingVar("BREVO_API_KEY"))?,
            owner_email: env::var("OWNER_EMAIL").unwrap_or_default(),
            security_template_id: env::var("BREVO_SECURITY_ALERT_TEMPLATE_ID").unwrap_or_else(|_| "9".to_string()),
            brevo_invite_template_id: env::var("BREVO_INVITE_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_INVITE_TEMPLATE_ID"))?,
            brevo_owner_reminder_early_template_id: env::var("BREVO_OWNER_REMINDER_EARLY_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_EARLY_TEMPLATE_ID"))?,
            brevo_owner_reminder_urgent_template_id: env::var("BREVO_OWNER_REMINDER_URGENT_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_URGENT_TEMPLATE_ID"))?,
            brevo_owner_reminder_daily_template_id: env::var("BREVO_OWNER_REMINDER_DAILY_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_DAILY_TEMPLATE_ID"))?,
            brevo_beneficiary_claim_available_template_id: env::var("BREVO_BENEFICIARY_CLAIM_AVAILABLE_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_BENEFICIARY_CLAIM_AVAILABLE_TEMPLATE_ID"))?,
            brevo_approver_attestation_request_template_id: env::var("BREVO_APPROVER_ATTESTATION_REQUEST_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_APPROVER_ATTESTATION_REQUEST_TEMPLATE_ID"))?,
            brevo_conflict_hold_notice_template_id: env::var("BREVO_CONFLICT_HOLD_NOTICE_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_CONFLICT_HOLD_NOTICE_TEMPLATE_ID"))?,
            brevo_release_ready_template_id: env::var("BREVO_RELEASE_READY_TEMPLATE_ID").map_err(|_| ConfigError::MissingVar("BREVO_RELEASE_READY_TEMPLATE_ID"))?,
            brevo_password_reset_template_id: env::var("BREVO_RESET_PASSWORD_TEMPLATE_ID").unwrap_or_else(|_| "9".to_string()),
        })
    }

    pub fn calculate_diff(&self, new: &Self) -> String {
        let mut diff = Vec::new();

        if self.openbao_version != new.openbao_version {
            diff.push(format!(
                "<b>Vault Version:</b> {} &rarr; {}",
                self.openbao_version, new.openbao_version
            ));
        }

        macro_rules! check_sensitive {
            ($name:expr, $old:expr, $new:expr) => {
                if $old != $new {
                    let old_hash = hash_value($old);
                    let new_hash = hash_value($new);
                    diff.push(format!(
                        "<b>{}:</b> [Hash: {}] &rarr; [Hash: {}]",
                        $name, old_hash, new_hash
                    ));
                }
            };
        }

        macro_rules! check_plain {
            ($name:expr, $old:expr, $new:expr) => {
                if $old != $new {
                    diff.push(format!("<b>{}:</b> {} &rarr; {}", $name, $old, $new));
                }
            };
        }

        check_sensitive!("SERVER_AEAD_KEY", &self.server_aead_key_b64, &new.server_aead_key_b64);
        check_sensitive!("OPAQUE_SERVER_SETUP", &self.opaque_server_setup_b64, &new.opaque_server_setup_b64);
        check_sensitive!("JWT_SECRET", &self.jwt_secret, &new.jwt_secret);
        check_sensitive!("SERVER_HMAC_SECRET", &self.server_hmac_secret, &new.server_hmac_secret);

        check_plain!("APP_URL", &self.app_url, &new.app_url);
        check_plain!("BRAND_NAME", &self.brand_name, &new.brand_name);
        
        let old_db = mask_url(&self.database_url);
        let new_db = mask_url(&new.database_url);
        if old_db != new_db {
            diff.push(format!("<b>DATABASE_URL:</b> {} &rarr; {}", old_db, new_db));
        }

        if diff.is_empty() {
            "No significant changes detected (possible soft-reload only).".to_string()
        } else {
            diff.join("<br/>")
        }
    }
}

fn hash_value(val: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(val.as_bytes());
    hex::encode(&hasher.finalize()[..4])
}

fn mask_url(url: &str) -> String {
    // Very simple masking for URLs (hiding pass)
    if let Some(at) = url.find('@') {
        if let Some(start) = url.find("://") {
            return format!("{}...{}", &url[..start + 3], &url[at..]);
        }
    }
    url.to_string()
}
