#![forbid(unsafe_code)]

use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use transfer_legacy_worker::config::Config;
use transfer_legacy_worker::queue::Queues;
use transfer_legacy_worker::scheduler::start_scheduler;
use transfer_legacy_worker::state::AppState;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    init_tracing();
    lock_memory_pages();
    tracing::info!("worker starting");

    let config = Config::from_env().map_err(|_| std::io::Error::other("config error"))?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .map_err(|_| std::io::Error::other("db error"))?;

    let queues = Queues::connect(&config.redis_url)
        .await
        .map_err(|_| std::io::Error::other("redis error"))?;

    let _state = AppState {
        config: config.clone(),
        db: db.clone(),
        heartbeat_storage: queues.heartbeat_storage.clone(),
        notify_storage: queues.notify_storage.clone(),
        audit_anchor_storage: queues.audit_anchor_storage.clone(),
        release_eval_storage: queues.release_eval_storage.clone(),
        conflict_check_storage: queues.conflict_check_storage.clone(),
        release_delivery_storage: queues.release_delivery_storage.clone(),
    };

    let _scheduler = start_scheduler(queues)
        .await
        .map_err(|_| std::io::Error::other("scheduler error"))?;

    tracing::info!("worker scheduler running");
    tokio::signal::ctrl_c()
        .await
        .map_err(|_| std::io::Error::other("signal error"))?;
    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}

fn lock_memory_pages() {
    #[cfg(unix)]
    {
        use nix::sys::mman::{MlockAllFlags, mlockall};
        if let Err(err) = mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
            tracing::warn!("mlockall unavailable: {err}");
        } else {
            tracing::info!("mlockall enabled");
        }
    }
}
