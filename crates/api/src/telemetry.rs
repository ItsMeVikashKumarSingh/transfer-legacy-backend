use metrics::gauge;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::OnceLock;
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

static PROM_HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn init_tracing() {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}

pub fn init_metrics() {
    if PROM_HANDLE.get().is_some() {
        return;
    }
    if let Ok(handle) = PrometheusBuilder::new().install_recorder() {
        let _ = PROM_HANDLE.set(handle);
        gauge!("server_decrypt_attempts_total").set(0.0);
    }
}

pub fn render_metrics() -> String {
    if let Some(handle) = PROM_HANDLE.get() {
        handle.render()
    } else {
        String::new()
    }
}
