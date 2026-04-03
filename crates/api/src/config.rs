use std::env;
use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub port: u16,
    pub allowed_origins: Vec<HeaderValue>,
    pub database_url: String,
    pub redis_url: String,
    pub server_aead_key_b64: String,
    pub opaque_server_setup_b64: String,
    pub jwt_secret: String,
    pub supabase_url: String,
    pub supabase_anon_key: String,
    pub supabase_service_role_key: String,
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

        Ok(Self {
            bind_addr,
            port,
            allowed_origins,
            database_url,
            redis_url,
            server_aead_key_b64,
            opaque_server_setup_b64,
            jwt_secret,
            supabase_url,
            supabase_anon_key,
            supabase_service_role_key,
        })
    }
}
