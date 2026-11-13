#![forbid(unsafe_code)]

use apalis::prelude::{Monitor, TokioExecutor, WorkerBuilder, WorkerFactoryFn};
use sqlx::postgres::PgPoolOptions;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

use transfer_legacy_worker::config::Config;
use transfer_legacy_worker::jobs;
use transfer_legacy_worker::queue::Queues;
use transfer_legacy_worker::scheduler::start_scheduler;
use transfer_legacy_worker::state::AppState;

#[tokio::main]
async fn main() -> Result<(), std::io::Error> {
    if let Err(e) = dotenvy::from_filename(".env.local") {
        if !matches!(e, dotenvy::Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound) {
            eprintln!("ERROR: Failed to parse .env.local file: {}", e);
            std::process::exit(1);
        }
    }
    if let Err(e) = dotenvy::dotenv() {
        if !matches!(e, dotenvy::Error::Io(ref io_err) if io_err.kind() == std::io::ErrorKind::NotFound) {
            eprintln!("ERROR: Failed to parse .env file: {}", e);
            std::process::exit(1);
        }
    }
    init_tracing();
    lock_memory_pages();
    tracing::info!("worker starting");

    // 1. Initial Load
    let config = if std::env::var("TL_ENV").unwrap_or_else(|_| "local".to_string()) == "local" {
        tracing::info!("Loading configuration from environment/dotenv...");
        match Config::from_env() {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("ERROR: Configuration error: {}", e);
                std::process::exit(1);
            }
        }
    } else {
        tracing::info!("Loading configuration from OpenBao (Environment: {})...", std::env::var("TL_ENV").unwrap_or_else(|_| "unknown".into()));
        match Config::load().await {
            Ok(cfg) => cfg,
            Err(e) => {
                eprintln!("ERROR: Failed to load configuration from OpenBao: {}", e);
                std::process::exit(1);
            }
        }
    };
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .map_err(|_| std::io::Error::other("db error"))?;

    let queues = Queues::connect(&config.redis_url)
        .await
        .map_err(|_| std::io::Error::other("redis error"))?;

    let state = AppState {
        config: config.clone(),
        db: db.clone(),
        heartbeat_storage: queues.heartbeat_storage.clone(),
        notify_storage: queues.notify_storage.clone(),
        audit_anchor_storage: queues.audit_anchor_storage.clone(),
        release_eval_storage: queues.release_eval_storage.clone(),
        conflict_check_storage: queues.conflict_check_storage.clone(),
        release_delivery_storage: queues.release_delivery_storage.clone(),
    };

    let _scheduler = start_scheduler(queues.clone())
        .await
        .map_err(|_| std::io::Error::other("scheduler error"))?;

    tracing::info!("worker scheduler running");
    let monitor = Monitor::<TokioExecutor>::new()
        .register_with_count(2, {
            WorkerBuilder::new("heartbeat-eval")
                .data(state.clone())
                .source(queues.heartbeat_storage.clone())
                .build_fn(jobs::run_heartbeat_eval)
        })
        .register_with_count(2, {
            WorkerBuilder::new("notify")
                .data(state.clone())
                .source(queues.notify_storage.clone())
                .build_fn(jobs::run_notify)
        })
        .register_with_count(1, {
            WorkerBuilder::new("audit-anchor")
                .data(state.clone())
                .source(queues.audit_anchor_storage.clone())
                .build_fn(jobs::run_audit_anchor)
        })
        .register_with_count(2, {
            WorkerBuilder::new("release-eval")
                .data(state.clone())
                .source(queues.release_eval_storage.clone())
                .build_fn(jobs::run_release_eval)
        })
        .register_with_count(2, {
            WorkerBuilder::new("conflict-check")
                .data(state.clone())
                .source(queues.conflict_check_storage.clone())
                .build_fn(jobs::run_conflict_check)
        })
        .register_with_count(2, {
            WorkerBuilder::new("release-delivery")
                .data(state.clone())
                .source(queues.release_delivery_storage.clone())
                .build_fn(jobs::run_release_delivery)
        });
    monitor
        .run()
        .await
        .map_err(|_| std::io::Error::other("worker monitor error"))?;
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
        use nix::sys::mman::{mlockall, MlockAllFlags};
        if let Err(err) = mlockall(MlockAllFlags::MCL_CURRENT | MlockAllFlags::MCL_FUTURE) {
            tracing::warn!("mlockall unavailable: {err}");
        } else {
            tracing::info!("mlockall enabled");
        }
    }
}
