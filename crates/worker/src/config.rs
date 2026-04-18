use reqwest::Client;
use serde::Deserialize;
use std::env;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Environment {
    Local,
    Staging,
    Production,
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local => write!(f, "local"),
            Self::Staging => write!(f, "staging"),
            Self::Production => write!(f, "production"),
        }
    }
}

impl From<&str> for Environment {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "staging" => Self::Staging,
            "production" | "prod" => Self::Production,
            _ => Self::Local,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub environment: Environment,
    pub database_url: String,
    pub redis_url: String,
    pub resend_api_key: String,
    pub bao_path: String,
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
    pub app_url: String,
    pub brand_name: String,
    pub redis_url: Option<String>,
    pub database_url: Option<String>,
    pub resend_api_key: String,
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
        let bao_path = env::var("BAO_PATH")
            .unwrap_or_else(|_| "secret/data/transfer-legacy/prod".to_string());
        let role_id = env::var("ROLE_ID").map_err(|_| ConfigError::MissingVar("ROLE_ID"))?;
        let secret_id = env::var("SECRET_ID").map_err(|_| ConfigError::MissingVar("SECRET_ID"))?;
        let server_id = env::var("TL_SERVER_ID").unwrap_or_else(|_| "worker-local".to_string());

        let tl_env_str = env::var("TL_ENV").unwrap_or_else(|_| "local".to_string());
        let environment = Environment::from(tl_env_str.as_str());

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

        let kv_url = format!("{}/v1/{}", openbao_addr, bao_path);
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
            environment,
            database_url: s.database_url.unwrap_or_default(),
            redis_url: s.redis_url.unwrap_or_default(),
            resend_api_key: s.resend_api_key,
            bao_path,
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

    pub fn from_env() -> Result<Self, ConfigError> {
        let tl_env_str = env::var("TL_ENV").unwrap_or_else(|_| "local".to_string());
        let environment = Environment::from(tl_env_str.as_str());
        let server_id = env::var("TL_SERVER_ID").unwrap_or_else(|_| "worker-local".to_string());

        Ok(Self {
            environment,
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingVar("DATABASE_URL"))?,
            redis_url: env::var("REDIS_URL").map_err(|_| ConfigError::MissingVar("REDIS_URL"))?,
            resend_api_key: env::var("RESEND_API_KEY")
                .map_err(|_| ConfigError::MissingVar("RESEND_API_KEY"))?,
            bao_path: env::var("BAO_PATH")
                .unwrap_or_else(|_| "secret/data/transfer-legacy/prod".to_string()),
            openbao_addr: env::var("OPENBAO_ADDR").unwrap_or_default(),
            openbao_token: "".to_string(),
            b2_key_id: env::var("B2_KEY_ID").unwrap_or_default(),
            b2_app_key: env::var("B2_APP_KEY").unwrap_or_default(),
            b2_bucket_name: env::var("B2_BUCKET_NAME").unwrap_or_default(),
            b2_audit_bucket_name: env::var("B2_AUDIT_BUCKET_NAME").unwrap_or_default(),
            b2_backup_bucket_name: env::var("B2_BACKUP_BUCKET_NAME").unwrap_or_default(),
            b2_endpoint_url: env::var("B2_ENDPOINT_URL").unwrap_or_default(),
            app_url: env::var("APP_URL").unwrap_or_else(|_| "http://localhost:3000".to_string()),
            brand_name: env::var("BRAND_NAME").unwrap_or_else(|_| "Transfer Legacy".into()),
            server_id,
            server_aead_key_b64: env::var("SERVER_AEAD_KEY_B64")
                .or_else(|_| env::var("SERVER_AEAD_KEY"))
                .map_err(|_| ConfigError::MissingVar("SERVER_AEAD_KEY_B64"))?,
        })
    }
}
