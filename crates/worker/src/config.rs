use std::env;
use serde::Deserialize;
use reqwest::Client;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub brevo_api_key: String,
    pub brevo_owner_reminder_early_template_id: String,
    pub brevo_owner_reminder_urgent_template_id: String,
    pub brevo_owner_reminder_daily_template_id: String,
    pub brevo_beneficiary_claim_template_id: String,
    pub brevo_approver_attestation_template_id: String,
    pub brevo_conflict_hold_template_id: String,
    pub brevo_release_ready_template_id: String,
    pub openbao_addr: String,
    pub openbao_token: String,
    pub b2_key_id: String,
    pub b2_app_key: String,
    pub b2_bucket_name: String,
    pub b2_audit_bucket_name: String,
    pub b2_backup_bucket_name: String,
    pub b2_endpoint_url: String,
    pub app_url: String,
    pub brand_name: String,
    pub server_id: String,
    pub server_aead_key_b64: String,
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

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing required env var: {0}")]
    MissingVar(&'static str),
    #[error("http error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("unexpected openbao response")]
    OpenBaoResponse,
}

impl Config {
    pub async fn load() -> Result<Self, ConfigError> {
        let openbao_addr =
            env::var("OPENBAO_ADDR").map_err(|_| ConfigError::MissingVar("OPENBAO_ADDR"))?;
        let role_id = env::var("ROLE_ID").map_err(|_| ConfigError::MissingVar("ROLE_ID"))?;
        let secret_id = env::var("SECRET_ID").map_err(|_| ConfigError::MissingVar("SECRET_ID"))?;
        let server_id = env::var("TL_SERVER_ID").unwrap_or_else(|_| "worker-local".to_string());

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
        }

        let body: VaultResponse = res.json().await?;
        let s = body.data.data;

        Ok(Self {
            database_url: s.database_url.unwrap_or_default(),
            redis_url: s.redis_url.unwrap_or_default(),
            brevo_api_key: s.brevo_api_key,
            brevo_owner_reminder_early_template_id: s.brevo_owner_reminder_early_template_id,
            brevo_owner_reminder_urgent_template_id: s.brevo_owner_reminder_urgent_template_id,
            brevo_owner_reminder_daily_template_id: s.brevo_owner_reminder_daily_template_id,
            brevo_beneficiary_claim_template_id: s.brevo_beneficiary_claim_available_template_id,
            brevo_approver_attestation_template_id: s.brevo_approver_attestation_request_template_id,
            brevo_conflict_hold_template_id: s.brevo_conflict_hold_notice_template_id,
            brevo_release_ready_template_id: s.brevo_release_ready_template_id,
            openbao_addr,
            openbao_token,
            b2_key_id: s.backblaze_b2_key_id,
            b2_app_key: s.backblaze_b2_app_key,
            b2_bucket_name: s.backblaze_b2_bucket_name,
            b2_audit_bucket_name: s.backblaze_b2_audit_bucket_name,
            b2_backup_bucket_name: s.backblaze_b2_backup_bucket_name,
            b2_endpoint_url: s.backblaze_b2_endpoint_url,
            app_url: s.app_url,
            brand_name: s.brand_name,
            server_id,
            server_aead_key_b64: s.server_aead_key,
        })
    }
}
