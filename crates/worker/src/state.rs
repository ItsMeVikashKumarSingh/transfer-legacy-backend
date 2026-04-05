use apalis_redis::RedisStorage;
use sqlx::PgPool;

use crate::config::Config;
use crate::jobs::{AuditAnchorJob, ConflictCheckJob, HeartbeatEvalJob, NotifyJob, ReleaseDeliveryJob, ReleaseEvalJob};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub heartbeat_storage: RedisStorage<HeartbeatEvalJob>,
    pub notify_storage: RedisStorage<NotifyJob>,
    pub audit_anchor_storage: RedisStorage<AuditAnchorJob>,
    pub release_eval_storage: RedisStorage<ReleaseEvalJob>,
    pub conflict_check_storage: RedisStorage<ConflictCheckJob>,
    pub release_delivery_storage: RedisStorage<ReleaseDeliveryJob>,
}
