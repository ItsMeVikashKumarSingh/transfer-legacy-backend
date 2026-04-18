use axum::http::HeaderValue as ax_http_header;
use reqwest::Client;
use serde::Deserialize;
use sha2::{Digest, Sha256};
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
    pub bind_addr: String,
    pub port: u16,
    pub allowed_origins: Vec<ax_http_header>,
    pub app_url: String,
    pub brand_name: String,
    pub internal_api_token: Option<String>,
    pub database_url: String,
    pub redis_url: String,
    pub bao_path: String,
    pub openbao_addr: String,
    pub openbao_token: String,
    pub openbao_version: u32,
    pub b2_key_id: String,
    pub b2_app_key: String,
    pub b2_bucket_name: String,
    pub b2_public_assets_bucket_name: String,
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
    pub resend_api_key: String,
    pub owner_email: String,
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
    #[serde(alias = "SERVER_AEAD_KEY_B64", alias = "SERVER_AEAD_KEY")]
    pub server_aead_key: String,
    #[serde(alias = "OPAQUE_SERVER_SETUP_B64", alias = "OPAQUE_SERVER_SETUP")]
    pub opaque_server_setup: String,
    #[serde(alias = "JWT_SECRET")]
    pub jwt_secret: String,
    #[serde(alias = "SERVER_HMAC_SECRET")]
    pub server_hmac_secret: String,
    #[serde(alias = "RESEND_API_KEY")]
    pub resend_api_key: String,
    #[serde(alias = "SUPABASE_URL")]
    pub supabase_url: String,
    #[serde(alias = "SUPABASE_PUBLISHABLE_KEY")]
    pub supabase_publishable_key: String,
    #[serde(alias = "SUPABASE_SECRET_KEY")]
    pub supabase_secret_key: String,
    #[serde(alias = "B2_KEY_ID", alias = "BACKBLAZE_B2_KEY_ID")]
    pub backblaze_b2_key_id: String,
    #[serde(alias = "B2_APP_KEY", alias = "BACKBLAZE_B2_APP_KEY")]
    pub backblaze_b2_app_key: String,
    #[serde(alias = "B2_BUCKET_NAME", alias = "BACKBLAZE_B2_BUCKET_NAME")]
    pub backblaze_b2_bucket_name: String,
    #[serde(alias = "B2_PUBLIC_ASSETS_BUCKET_NAME", alias = "BACKBLAZE_B2_PUBLIC_ASSETS_BUCKET_NAME")]
    pub backblaze_b2_public_assets_bucket_name: String,
    #[serde(alias = "B2_AUDIT_BUCKET_NAME", alias = "BACKBLAZE_B2_AUDIT_BUCKET_NAME")]
    pub backblaze_b2_audit_bucket_name: String,
    #[serde(alias = "B2_BACKUP_BUCKET_NAME", alias = "BACKBLAZE_B2_BACKUP_BUCKET_NAME")]
    pub backblaze_b2_backup_bucket_name: String,
    #[serde(alias = "B2_ENDPOINT_URL", alias = "BACKBLAZE_B2_ENDPOINT_URL")]
    pub backblaze_b2_endpoint_url: String,
    #[serde(alias = "INTERNAL_API_TOKEN")]
    pub internal_api_token: Option<String>,
    #[serde(alias = "OWNER_EMAIL")]
    pub owner_email: String,
    #[serde(alias = "APP_URL")]
    pub app_url: String,
    #[serde(alias = "BRAND_NAME")]
    pub brand_name: String,
    #[serde(alias = "REDIS_URL")]
    pub redis_url: Option<String>,
    #[serde(alias = "DATABASE_URL")]
    pub database_url: Option<String>,
}

impl Config {
    pub async fn load() -> Result<Self, ConfigError> {
        // 1. Gather Bootstrap Env Vars
        let bind_addr = env::var("TL_BIND_ADDR").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port_str = env::var("TL_PORT").unwrap_or_else(|_| "8080".to_string());
        let port = port_str
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort(port_str))?;

        let tl_env_str = env::var("TL_ENV").unwrap_or_else(|_| "local".to_string());
        let environment = Environment::from(tl_env_str.as_str());

        let openbao_addr =
            env::var("OPENBAO_ADDR").map_err(|_| ConfigError::MissingVar("OPENBAO_ADDR"))?;
        let bao_path = env::var("BAO_PATH")
            .unwrap_or_else(|_| "secret/data/transfer-legacy/prod".to_string());
        let role_id = env::var("ROLE_ID").map_err(|_| ConfigError::MissingVar("ROLE_ID"))?;
        let secret_id = env::var("SECRET_ID").map_err(|_| ConfigError::MissingVar("SECRET_ID"))?;

        // 2. Authenticate with AppRole to get a token
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
            metadata: VaultMetadata,
        }
        #[derive(Deserialize)]
        struct VaultMetadata {
            version: u32,
        }

        let body: VaultResponse = res.json().await?;
        let s = body.data.data;
        let openbao_version = body.data.metadata.version;

        let allowed_origins = vec![ax_http_header::from_str(s.app_url.trim()).unwrap()];

        tracing::info!("Config loaded from OpenBao. AEAD Key Hash: {}", hash_value(s.server_aead_key.trim()));
        println!("🚀 [DIAGNOSTIC] Config loaded from OpenBao. AEAD Key Hash: {}", hash_value(s.server_aead_key.trim()));

        Ok(Self {
            environment,
            bind_addr,
            port,
            allowed_origins,
            app_url: s.app_url.trim().to_string(),
            brand_name: s.brand_name.trim().to_string(),
            internal_api_token: s.internal_api_token.map(|t| t.trim().to_string()),
            database_url: s.database_url.unwrap_or_default().trim().to_string(),
            redis_url: s.redis_url.unwrap_or_default().trim().to_string(),
            bao_path,
            openbao_addr,
            openbao_token,
            openbao_version,
            b2_key_id: s.backblaze_b2_key_id.trim().to_string(),
            b2_app_key: s.backblaze_b2_app_key.trim().to_string(),
            b2_bucket_name: s.backblaze_b2_bucket_name.trim().to_string(),
            b2_public_assets_bucket_name: s.backblaze_b2_public_assets_bucket_name.trim().to_string(),
            b2_audit_bucket_name: s.backblaze_b2_audit_bucket_name.trim().to_string(),
            b2_backup_bucket_name: s.backblaze_b2_backup_bucket_name.trim().to_string(),
            b2_endpoint_url: s.backblaze_b2_endpoint_url.trim().to_string(),
            server_aead_key_b64: s.server_aead_key.trim().to_string(),
            opaque_server_setup_b64: s.opaque_server_setup.trim().to_string(),
            jwt_secret: s.jwt_secret.trim().to_string(),
            supabase_url: s.supabase_url.trim().to_string(),
            supabase_publishable_key: s.supabase_publishable_key.trim().to_string(),
            supabase_secret_key: s.supabase_secret_key.trim().to_string(),
            server_hmac_secret: s.server_hmac_secret.trim().to_string(),
            resend_api_key: s.resend_api_key.trim().to_string(),
            owner_email: s.owner_email.trim().to_string(),
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

        let tl_env_str = env::var("TL_ENV").unwrap_or_else(|_| "local".to_string());
        let environment = Environment::from(tl_env_str.as_str());

        let app_url = env::var("TL_APP_URL").or_else(|_| env::var("APP_URL")).unwrap_or_else(|_| "http://localhost:3000".to_string()).trim().to_string();
        let allowed_origins = vec![ax_http_header::from_str(&app_url).unwrap()];
        
        let aead_key = env::var("TL_SERVER_AEAD_KEY_B64")
            .or_else(|_| env::var("SERVER_AEAD_KEY_B64"))
            .or_else(|_| env::var("SERVER_AEAD_KEY"))
            .map_err(|_| ConfigError::MissingVar("SERVER_AEAD_KEY_B64"))?
            .trim()
            .to_string();

        tracing::info!("Config loaded from Env. AEAD Key Hash: {}", hash_value(&aead_key));

        Ok(Self {
            environment,
            bind_addr,
            port,
            allowed_origins,
            app_url,
            brand_name: env::var("TL_BRAND_NAME").or_else(|_| env::var("BRAND_NAME")).unwrap_or_else(|_| "Transfer Legacy".into()).trim().to_string(),
            internal_api_token: env::var("TL_INTERNAL_API_TOKEN").or_else(|_| env::var("INTERNAL_API_TOKEN")).ok().map(|t| t.trim().to_string()),
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingVar("DATABASE_URL"))?
                .trim()
                .to_string(),
            redis_url: env::var("REDIS_URL").map_err(|_| ConfigError::MissingVar("REDIS_URL"))?
                .trim()
                .to_string(),
            bao_path: env::var("BAO_PATH")
                .unwrap_or_else(|_| "secret/data/transfer-legacy/prod".to_string())
                .trim()
                .to_string(),
            openbao_addr: env::var("OPENBAO_ADDR").unwrap_or_default().trim().to_string(),
            openbao_token: "".to_string(),
            openbao_version: 0,
            b2_key_id: env::var("B2_KEY_ID").unwrap_or_default().trim().to_string(),
            b2_app_key: env::var("B2_APP_KEY").unwrap_or_default().trim().to_string(),
            b2_bucket_name: env::var("B2_BUCKET_NAME").unwrap_or_default().trim().to_string(),
            b2_public_assets_bucket_name: env::var("B2_PUBLIC_ASSETS_BUCKET_NAME").unwrap_or_default().trim().to_string(),
            b2_audit_bucket_name: env::var("B2_AUDIT_BUCKET_NAME").unwrap_or_default().trim().to_string(),
            b2_backup_bucket_name: env::var("B2_BACKUP_BUCKET_NAME").unwrap_or_default().trim().to_string(),
            b2_endpoint_url: env::var("B2_ENDPOINT_URL").unwrap_or_default().trim().to_string(),
            server_aead_key_b64: aead_key,
            opaque_server_setup_b64: env::var("OPAQUE_SERVER_SETUP_B64")
                .or_else(|_| env::var("OPAQUE_SERVER_SETUP"))
                .map_err(|_| ConfigError::MissingVar("OPAQUE_SERVER_SETUP_B64"))?
                .trim()
                .to_string(),
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| ConfigError::MissingVar("JWT_SECRET"))?
                .trim()
                .to_string(),
            supabase_url: env::var("SUPABASE_URL")
                .map_err(|_| ConfigError::MissingVar("SUPABASE_URL"))?
                .trim()
                .to_string(),
            supabase_publishable_key: env::var("SUPABASE_PUBLISHABLE_KEY")
                .map_err(|_| ConfigError::MissingVar("SUPABASE_PUBLISHABLE_KEY"))?
                .trim()
                .to_string(),
            supabase_secret_key: env::var("SUPABASE_SECRET_KEY")
                .map_err(|_| ConfigError::MissingVar("SUPABASE_SECRET_KEY"))?
                .trim()
                .to_string(),
            server_hmac_secret: env::var("SERVER_HMAC_SECRET")
                .map_err(|_| ConfigError::MissingVar("SERVER_HMAC_SECRET"))?
                .trim()
                .to_string(),
            resend_api_key: env::var("RESEND_API_KEY")
                .map_err(|_| ConfigError::MissingVar("RESEND_API_KEY"))?
                .trim()
                .to_string(),
            owner_email: env::var("OWNER_EMAIL").unwrap_or_default().trim().to_string(),
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

        check_sensitive!(
            "SERVER_AEAD_KEY",
            &self.server_aead_key_b64,
            &new.server_aead_key_b64
        );
        check_sensitive!(
            "OPAQUE_SERVER_SETUP",
            &self.opaque_server_setup_b64,
            &new.opaque_server_setup_b64
        );
        check_sensitive!("JWT_SECRET", &self.jwt_secret, &new.jwt_secret);
        check_sensitive!(
            "SERVER_HMAC_SECRET",
            &self.server_hmac_secret,
            &new.server_hmac_secret
        );

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
    if let Some(at) = url.find('@') {
        if let Some(start) = url.find("://") {
            return format!("{}...{}", &url[..start + 3], &url[at..]);
        }
    }
    url.to_string()
}
