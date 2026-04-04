#![forbid(unsafe_code)]

use apalis::prelude::*;
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use transfer_legacy_worker::config::Config;
use transfer_legacy_worker::dlq::record_failed_job;
use transfer_legacy_worker::jobs::{run_audit_anchor, run_heartbeat_eval, run_notify, run_release_eval, AuditAnchorJob, HeartbeatEvalJob, NotifyJob, ReleaseEvalJob};
use transfer_legacy_worker::queue::Queues;
use transfer_legacy_worker::scheduler::start_scheduler;
use transfer_legacy_worker::state::AppState;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_tracing();
    tracing::info!("worker starting");

    let config = Config::from_env().map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "config error"))?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "db error"))?;

    let queues = Queues::connect(&config.redis_url)
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "redis error"))?;

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        heartbeat_storage: queues.heartbeat_storage.clone(),
        notify_storage: queues.notify_storage.clone(),
        audit_anchor_storage: queues.audit_anchor_storage.clone(),
        release_eval_storage: queues.release_eval_storage.clone(),
    };

    let _scheduler = start_scheduler(queues.clone())
        .await
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, "scheduler error"))?;

    let heartbeat_worker = WorkerBuilder::new("heartbeat-eval")
        .data(state.clone())
        .with_storage(queues.heartbeat_storage.clone())
        .build(|job: HeartbeatEvalJob, state: Data<AppState>| async move {
            let result = run_heartbeat_eval(job.clone(), state.clone()).await;
            if let Err(err) = result {
                let attempts = job.attempts + 1;
                if attempts >= 3 {
                    let _ = record_failed_job(&state.db, "HeartbeatEvalJob", &job, &format!("{:?}", err), attempts as i32).await;
                    return Ok(());
                }
                let retry = HeartbeatEvalJob { attempts };
                let _ = state.heartbeat_storage.push(retry).await;
            }
            Ok(())
        });

    let notify_worker = WorkerBuilder::new("notify")
        .data(state.clone())
        .with_storage(queues.notify_storage.clone())
        .build(|job: NotifyJob, state: Data<AppState>| async move {
            let result = run_notify(job.clone(), state.clone()).await;
            if let Err(err) = result {
                let attempts = job.attempts + 1;
                if attempts >= 3 {
                    let _ = record_failed_job(&state.db, "NotifyJob", &job, &format!("{:?}", err), attempts as i32).await;
                    return Ok(());
                }
                let mut retry = job.clone();
                retry.attempts = attempts;
                let _ = state.notify_storage.push(retry).await;
            }
            Ok(())
        });

    let audit_anchor_worker = WorkerBuilder::new("audit-anchor")
        .data(state.clone())
        .with_storage(queues.audit_anchor_storage.clone())
        .build(|job: AuditAnchorJob, state: Data<AppState>| async move {
            let result = run_audit_anchor(job.clone(), state.clone()).await;
            if let Err(err) = result {
                let attempts = job.attempts + 1;
                if attempts >= 3 {
                    let _ = record_failed_job(&state.db, "AuditAnchorJob", &job, &format!("{:?}", err), attempts as i32).await;
                    return Ok(());
                }
                let retry = AuditAnchorJob { date: job.date, attempts };
                let _ = state.audit_anchor_storage.push(retry).await;
            }
            Ok(())
        });

    let release_eval_worker = WorkerBuilder::new("release-eval")
        .data(state.clone())
        .with_storage(queues.release_eval_storage.clone())
        .build(|job: ReleaseEvalJob, state: Data<AppState>| async move {
            let result = run_release_eval(job.clone(), state.clone()).await;
            if let Err(err) = result {
                let attempts = job.attempts + 1;
                if attempts >= 3 {
                    let _ = record_failed_job(&state.db, "ReleaseEvalJob", &job, &format!("{:?}", err), attempts as i32).await;
                    return Ok(());
                }
                let retry = ReleaseEvalJob { attempts };
                let _ = state.release_eval_storage.push(retry).await;
            }
            Ok(())
        });

    tokio::select! {
        _ = heartbeat_worker.run() => {},
        _ = notify_worker.run() => {},
        _ = audit_anchor_worker.run() => {},
        _ = release_eval_worker.run() => {},
    }

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}
