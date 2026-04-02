use std::env;
use axum::http::HeaderValue;

#[derive(Debug, Clone)]
pub struct Config {
    pub bind_addr: String,
    pub port: u16,
    pub allowed_origins: Vec<HeaderValue>,
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
            .map_err(|_| ConfigError::MissingVar(\"TL_ALLOWED_ORIGINS\"))?;

        if allowed_origins.is_empty() {
            return Err(ConfigError::MissingVar("TL_ALLOWED_ORIGINS"));
        }

        Ok(Self {
            bind_addr,
            port,
            allowed_origins,
        })
    }
}
