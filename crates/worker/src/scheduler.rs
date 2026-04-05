use tokio_cron_scheduler::{Job, JobScheduler};

use crate::jobs::{AuditAnchorJob, ConflictCheckJob, HeartbeatEvalJob, ReleaseDeliveryJob, ReleaseEvalJob};
use crate::queue::Queues;
use chrono::Utc;

#[derive(thiserror::Error, Debug)]
pub enum SchedulerError {
    #[error("scheduler error")]
    Scheduler,
}

pub async fn start_scheduler(queues: Queues) -> Result<JobScheduler, SchedulerError> {
    let sched = JobScheduler::new().await.map_err(|_| SchedulerError::Scheduler)?;

    let heartbeat_storage = queues.heartbeat_storage.clone();
    let heartbeat_job = Job::new_async("0 0 * * * *", move |_id, _lock| {
        let storage = heartbeat_storage.clone();
        Box::pin(async move {
            let _ = storage.push(HeartbeatEvalJob { attempts: 0 }).await;
        })
    })
    .map_err(|_| SchedulerError::Scheduler)?;
    sched.add(heartbeat_job).map_err(|_| SchedulerError::Scheduler)?;

    let anchor_storage = queues.audit_anchor_storage.clone();
    let anchor_job = Job::new_async("0 5 0 * * *", move |_id, _lock| {
        let storage = anchor_storage.clone();
        Box::pin(async move {
            let date = Utc::now().date_naive();
            let _ = storage.push(AuditAnchorJob { date, attempts: 0 }).await;
        })
    })
    .map_err(|_| SchedulerError::Scheduler)?;
    sched.add(anchor_job).map_err(|_| SchedulerError::Scheduler)?;

    let release_storage = queues.release_eval_storage.clone();
    let release_job = Job::new_async("0 10 * * * *", move |_id, _lock| {
        let storage = release_storage.clone();
        Box::pin(async move {
            let _ = storage.push(ReleaseEvalJob { attempts: 0 }).await;
        })
    })
    .map_err(|_| SchedulerError::Scheduler)?;
    sched.add(release_job).map_err(|_| SchedulerError::Scheduler)?;

    let conflict_storage = queues.conflict_check_storage.clone();
    let conflict_job = Job::new_async("0 15 * * * *", move |_id, _lock| {
        let storage = conflict_storage.clone();
        Box::pin(async move {
            let _ = storage.push(ConflictCheckJob { attempts: 0 }).await;
        })
    })
    .map_err(|_| SchedulerError::Scheduler)?;
    sched.add(conflict_job).map_err(|_| SchedulerError::Scheduler)?;

    let delivery_storage = queues.release_delivery_storage.clone();
    let delivery_job = Job::new_async("0 20 * * * *", move |_id, _lock| {
        let storage = delivery_storage.clone();
        Box::pin(async move {
            let _ = storage.push(ReleaseDeliveryJob { attempts: 0 }).await;
        })
    })
    .map_err(|_| SchedulerError::Scheduler)?;
    sched.add(delivery_job).map_err(|_| SchedulerError::Scheduler)?;

    sched.start().await.map_err(|_| SchedulerError::Scheduler)?;

    Ok(sched)
}
