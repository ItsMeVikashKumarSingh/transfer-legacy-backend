use std::env;
use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub port: u16,
    pub allowed_origins: Vec<HeaderValue>,
    pub app_url: String,
    pub brand_name: String,
    pub database_url: String,
    pub redis_url: String,
    pub server_aead_key_b64: String,
    pub opaque_server_setup_b64: String,
    pub jwt_secret: String,
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
    pub server_hmac_secret: String,
    pub brevo_api_key: String,
    pub brevo_invite_template_id: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing required env var: {0}")]
    MissingVar(&'static str),
    #[error("invalid port in TL_PORT: {0}")]
    InvalidPort(String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let bind_addr = env::var("TL_BIND_ADDR")
            .map_err(|_| ConfigError::MissingVar("TL_BIND_ADDR"))?;
        let port_str = env::var("TL_PORT").map_err(|_| ConfigError::MissingVar("TL_PORT"))?;
        let port = port_str
            .parse::<u16>()
            .map_err(|_| ConfigError::InvalidPort(port_str))?;
        let origins_raw = env::var("TL_ALLOWED_ORIGINS")
            .map_err(|_| ConfigError::MissingVar("TL_ALLOWED_ORIGINS"))?;
        let allowed_origins = origins_raw
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(HeaderValue::from_str)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ConfigError::MissingVar("TL_ALLOWED_ORIGINS"))?;

        if allowed_origins.is_empty() {
            return Err(ConfigError::MissingVar("TL_ALLOWED_ORIGINS"));
        }

        let app_url = env::var("TL_APP_URL").map_err(|_| ConfigError::MissingVar("TL_APP_URL"))?;
        let brand_name = env::var("TL_BRAND_NAME").map_err(|_| ConfigError::MissingVar("TL_BRAND_NAME"))?;
        let database_url = env::var("DATABASE_URL")
            .map_err(|_| ConfigError::MissingVar("DATABASE_URL"))?;
        let redis_url = env::var("REDIS_URL").map_err(|_| ConfigError::MissingVar("REDIS_URL"))?;
        let server_aead_key_b64 = env::var("SERVER_AEAD_KEY")
            .map_err(|_| ConfigError::MissingVar("SERVER_AEAD_KEY"))?;
        let opaque_server_setup_b64 = env::var("OPAQUE_SERVER_SETUP")
            .map_err(|_| ConfigError::MissingVar("OPAQUE_SERVER_SETUP"))?;
        let jwt_secret = env::var("JWT_SECRET")
            .map_err(|_| ConfigError::MissingVar("JWT_SECRET"))?;
        let supabase_url = env::var("SUPABASE_URL")
            .map_err(|_| ConfigError::MissingVar("SUPABASE_URL"))?;
        let supabase_anon_key = env::var("SUPABASE_ANON_KEY")
            .map_err(|_| ConfigError::MissingVar("SUPABASE_ANON_KEY"))?;
        let supabase_service_role_key = env::var("SUPABASE_SERVICE_ROLE_KEY")
            .map_err(|_| ConfigError::MissingVar("SUPABASE_SERVICE_ROLE_KEY"))?;
        let server_hmac_secret = env::var("SERVER_HMAC_SECRET")
            .map_err(|_| ConfigError::MissingVar("SERVER_HMAC_SECRET"))?;
        let brevo_api_key = env::var("BREVO_API_KEY")
            .map_err(|_| ConfigError::MissingVar("BREVO_API_KEY"))?;
        let brevo_invite_template_id = env::var("BREVO_INVITE_TEMPLATE_ID")
            .map_err(|_| ConfigError::MissingVar("BREVO_INVITE_TEMPLATE_ID"))?;

        Ok(Self {
            bind_addr,
            port,
            allowed_origins,
            app_url,
            brand_name,
            database_url,
            redis_url,
            server_aead_key_b64,
            opaque_server_setup_b64,
            jwt_secret,
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key,
            server_hmac_secret,
            brevo_api_key,
            brevo_invite_template_id,
        })
    }
}
