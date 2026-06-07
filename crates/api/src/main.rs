#![forbid(unsafe_code)]

mod config;
mod db;
mod errors;
mod handlers;
mod middleware;
mod notifications;
mod router;
mod services;
mod signing;
mod state;
mod storage;
mod telemetry;
#[cfg(test)]
mod tests;

use crate::config::Config;
use crate::errors::ApiError;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
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
    telemetry::init_tracing();
    telemetry::init_metrics();

    // 1. Initial Load
    let config = if std::env::var("TL_ENV").unwrap_or_else(|_| "local".to_string()) == "local" {
        tracing::info!("Loading configuration from environment/dotenv...");
        match Config::from_env() {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!("ERROR: Configuration error: {}", e);
                use std::io::Write;
                std::io::stdout().flush().ok();
                std::io::stderr().flush().ok();
                panic!("CRITICAL: Configuration error during startup: {}", e);
            }
        }
    } else {
        tracing::info!("Loading configuration from OpenBao (Environment: {})...", std::env::var("TL_ENV").unwrap_or_else(|_| "unknown".into()));
        match Config::load().await {
            Ok(cfg) => cfg,
            Err(e) => {
                tracing::error!("ERROR: Failed to load configuration: {}", e);
                use std::io::Write;
                std::io::stdout().flush().ok();
                std::io::stderr().flush().ok();
                panic!("CRITICAL: Failed to load configuration during startup: {}", e);
            }
        }
    };
    let state = match state::AppState::new(config.clone()).await {
        Ok(s) => s,
        Err(e) => {
            tracing::error!("ERROR: Failed to initialize application state: {}", e);
            use std::io::Write;
            std::io::stdout().flush().ok();
            std::io::stderr().flush().ok();
            panic!("CRITICAL: Failed to initialize application state during startup: {}", e);
        }
    };

    // 2. Setup Hot-Reload Listener (SIGHUP)
    #[cfg(unix)]
    let state_for_reload = state.clone();
    #[cfg(unix)]
    tokio::spawn(async move {
        use tokio::signal::unix::{signal, SignalKind};

        let mut stream = match signal(SignalKind::hangup()) {
            Ok(s) => s,
            Err(e) => {
                tracing::error!("Failed to setup SIGHUP listener: {}", e);
                return;
            }
        };

        while stream.recv().await.is_some() {
            tracing::info!("♻️ SIGHUP received: Reloading configuration from OpenBao...");
            let old_config = state_for_reload.config.read().await.clone();

            match Config::load().await {
                Ok(new_config) => {
                    let diff_html = old_config.calculate_diff(&new_config);
                    let audit_details = format!(
                        "Source: SIGHUP Signal (Hot-Reload)<br/>Timestamp: {}<br/>Environment: Production",
                        chrono::Utc::now().to_rfc3339()
                    );

                    let mut lock = state_for_reload.config.write().await;
                    *lock = new_config.clone();
                    tracing::info!("✅ Configuration reloaded successfully.");

                    // Send Security Notification
                    if let Err(e) = crate::notifications::resend::send_notification(
                        &new_config,
                        &new_config.owner_email,
                        crate::notifications::resend::NotificationTemplate::SecurityAlert {
                            diff_html,
                            audit_details,
                        },
                    )
                    .await
                    {
                        tracing::error!("❌ Failed to send security notification: {:?}", e);
                    }
                }
                Err(e) => {
                    tracing::error!("❌ Failed to reload configuration: {}", e);
                }
            }
        }
    });

    #[cfg(not(unix))]
    tracing::warn!("SIGHUP hot-reloading is only supported on Unix-like systems.");

    // 3. Start Server
    let config_lock = state.config.read().await;
    let app = router::create_router(&config_lock, state.clone());
    let bind_addr = format!("{}:{}", config_lock.bind_addr, config_lock.port);
    drop(config_lock); // Release lock before serve

    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;
    tracing::info!("listening on {bind_addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
