use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use metrics::{counter, histogram};
use std::time::Instant;

pub async fn metrics_middleware(request: Request, next: Next) -> Response {
    let route = request.uri().path().to_string();
    let start = Instant::now();
    let response = next.run(request).await;
    let status = response.status().as_u16().to_string();
    let elapsed = start.elapsed().as_secs_f64();

    histogram!("api_request_duration_seconds", "route" => route.clone(), "status" => status.clone()).record(elapsed);
    if status.starts_with('4') || status.starts_with('5') {
        tracing::error!("API Request to {} failed with status {}", route, status);
        counter!("api_errors_total", "route" => route, "status" => status).increment(1);
    }
    response
}
