use apalis_redis::RedisStorage;

use crate::jobs::{
    AuditAnchorJob, ConflictCheckJob, HeartbeatEvalJob, NotifyJob, ReleaseDeliveryJob,
    ReleaseEvalJob,
};

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
    pub async fn connect(redis_url: &str) -> Result<Self, redis::RedisError> {
        let conn = apalis_redis::connect(redis_url).await?;
        Ok(Self {
            heartbeat_storage: RedisStorage::new(conn.clone()),
            notify_storage: RedisStorage::new(conn.clone()),
            audit_anchor_storage: RedisStorage::new(conn.clone()),
            release_eval_storage: RedisStorage::new(conn.clone()),
            conflict_check_storage: RedisStorage::new(conn.clone()),
            release_delivery_storage: RedisStorage::new(conn),
        })
    }
}
