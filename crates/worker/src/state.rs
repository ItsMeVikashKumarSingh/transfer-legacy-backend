use apalis_redis::RedisStorage;
use sqlx::PgPool;

use crate::config::Config;
use crate::jobs::{AuditAnchorJob, HeartbeatEvalJob, NotifyJob, ReleaseEvalJob};

#[derive(Clone)]
pub struct AppState {
    pub config: Config,
    pub db: PgPool,
    pub heartbeat_storage: RedisStorage<HeartbeatEvalJob>,
    pub notify_storage: RedisStorage<NotifyJob>,
    pub audit_anchor_storage: RedisStorage<AuditAnchorJob>,
    pub release_eval_storage: RedisStorage<ReleaseEvalJob>,
}
