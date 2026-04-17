use redis::Client as RedisClient;
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use transfer_legacy_crypto_core::opaque::{server_setup_from_b64, OpaqueError, OpaqueServerSetup};

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RwLock<Config>>,
    pub db: PgPool,
    pub redis: RedisClient,
    pub opaque_setup: OpaqueServerSetup,
}

#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("database pool error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("redis client error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("opaque setup error: {0}")]
    Opaque(#[from] OpaqueError),
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, StateError> {
        let db = PgPool::connect(&config.database_url).await?;
        let redis = RedisClient::open(config.redis_url.as_str())?;
        let opaque_setup = server_setup_from_b64(&config.opaque_server_setup_b64)?;

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            db,
            redis,
            opaque_setup,
        })
    }

    /// Pull a cloned snapshot of the current config.
    pub async fn config(&self) -> Config {
        self.config.read().await.clone()
    }

    pub fn db(&self) -> &PgPool {
        &self.db
    }

    pub async fn notify(
        &self,
        _user_id: uuid::Uuid,
        to_email: &str,
        template: crate::notifications::resend::NotificationTemplate,
    ) -> anyhow::Result<()> {
        let config = self.config().await;
        crate::notifications::resend::send_notification(&config, to_email, template).await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
