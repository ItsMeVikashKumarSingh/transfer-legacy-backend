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
    pub redis_conn: redis::aio::ConnectionManager,
    pub opaque_setup: OpaqueServerSetup,
    pub signer: Arc<dyn crate::services::signing::TransitSigner>,
}

#[derive(thiserror::Error, Debug)]
pub enum StateError {
    #[error("database pool error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("redis client error: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("opaque setup error: {0}")]
    Opaque(#[from] OpaqueError),
    #[error("signer setup error: {0}")]
    Signer(String),
}

impl AppState {
    pub async fn new(config: Config) -> Result<Self, StateError> {
        let db = sqlx::postgres::PgPoolOptions::new()
            .max_connections(3)
            .min_connections(0)
            .acquire_timeout(std::time::Duration::from_secs(3))
            .idle_timeout(std::time::Duration::from_secs(30))
            .connect_lazy(&config.database_url)?;
        let redis = RedisClient::open(config.redis_url.as_str())?;


        let redis_conn = redis.get_connection_manager().await?;
        let opaque_setup = server_setup_from_b64(&config.opaque_server_setup_b64)?;

        let signer: Arc<dyn crate::services::signing::TransitSigner> = if config.tl_serverless {
            let key = config.server_private_key_b64.as_deref().unwrap_or_default();
            let s = crate::services::signing::InMemorySigner::new(key)
                .map_err(|e| StateError::Signer(e.to_string()))?;
            Arc::new(s)
        } else {
            let s = crate::services::signing::OpenBaoSigner::new(
                config.openbao_addr.clone(),
                config.openbao_token.clone(),
            );
            Arc::new(s)
        };

        Ok(Self {
            config: Arc::new(RwLock::new(config)),
            db,
            redis,
            redis_conn,
            opaque_setup,
            signer,
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
        crate::notifications::resend::send_notification(&config, to_email, template)
            .await
            .map_err(|e| anyhow::anyhow!(e))
    }
}
