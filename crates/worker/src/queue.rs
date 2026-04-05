use apalis_redis::RedisStorage;

use crate::jobs::{AuditAnchorJob, ConflictCheckJob, HeartbeatEvalJob, NotifyJob, ReleaseDeliveryJob, ReleaseEvalJob};

#[derive(Clone)]
pub struct Queues {
    pub heartbeat_storage: RedisStorage<HeartbeatEvalJob>,
    pub notify_storage: RedisStorage<NotifyJob>,
    pub audit_anchor_storage: RedisStorage<AuditAnchorJob>,
    pub release_eval_storage: RedisStorage<ReleaseEvalJob>,
    pub conflict_check_storage: RedisStorage<ConflictCheckJob>,
    pub release_delivery_storage: RedisStorage<ReleaseDeliveryJob>,
}

impl Queues {
    pub async fn connect(redis_url: &str) -> Result<Self, apalis_redis::RedisError> {
        Ok(Self {
            heartbeat_storage: RedisStorage::connect(redis_url).await?,
            notify_storage: RedisStorage::connect(redis_url).await?,
            audit_anchor_storage: RedisStorage::connect(redis_url).await?,
            release_eval_storage: RedisStorage::connect(redis_url).await?,
            conflict_check_storage: RedisStorage::connect(redis_url).await?,
            release_delivery_storage: RedisStorage::connect(redis_url).await?,
        })
    }
}
