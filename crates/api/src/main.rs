#![forbid(unsafe_code)]

mod config;
mod errors;
mod handlers;
mod middleware;
mod router;
mod telemetry;
mod services;
mod db;
mod signing;
mod storage;
mod notifications;
mod state;
#[cfg(test)]
mod tests;

use crate::config::Config;
use crate::errors::ApiError;

#[tokio::main]
async fn main() -> Result<(), ApiError> {
    dotenvy::from_filename(".env.local").ok();
    dotenvy::dotenv().ok();
    telemetry::init_tracing();
    telemetry::init_metrics();

    let config = Config::from_env()?;
    let state = state::AppState::new(config.clone()).await?;
    let app = router::create_router(&config, state);
    let bind_addr = format!("{}:{}", config.bind_addr, config.port);
    let listener = tokio::net::TcpListener::bind(&bind_addr).await?;

    tracing::info!("listening on {bind_addr}");

    axum::serve(listener, app).await?;
    Ok(())
}
