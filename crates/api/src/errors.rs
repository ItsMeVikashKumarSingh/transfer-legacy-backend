use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use metrics::counter;
use serde::Serialize;
use tower_http::request_id::RequestId;

use transfer_legacy_shared_types::AppError;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("server error: {0}")]
    Server(#[from] hyper::Error),
    #[error("state error: {0}")]
    State(#[from] crate::state::StateError),
    #[error("app error: {0}")]
    App(AppError),
    #[error("app error with request id: {0}")]
    AppWithRequestId(AppError, String),
}

#[derive(Serialize)]
struct ErrorBody {
    code: String,
    message: String,
    request_id: String,
}

#[derive(Serialize)]
struct ErrorEnvelope {
    error: ErrorBody,
}

#[derive(Serialize, serde::Deserialize)]
pub struct SuccessEnvelope<T> {
    pub data: T,
    pub request_id: String,
}

pub fn success<T: Serialize>(request_id: &str, data: T) -> Json<SuccessEnvelope<T>> {
    Json(SuccessEnvelope {
        data,
        request_id: request_id.to_string(),
    })
}

impl ApiError {
    pub fn app(err: AppError) -> Self {
        ApiError::App(err)
    }

    pub fn app_with_request_id(err: AppError, request_id: &str) -> Self {
        ApiError::AppWithRequestId(err, request_id.to_string())
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, app_error, request_id) = match self {
            ApiError::App(err) => (status_for(&err), err, "unknown".to_string()),
            ApiError::AppWithRequestId(err, request_id) => (status_for(&err), err, request_id),
            _ => (
                StatusCode::INTERNAL_SERVER_ERROR,
                AppError::Internal,
                "unknown".to_string(),
            ),
        };

        let body = ErrorEnvelope {
            error: ErrorBody {
                code: app_error.code().to_string(),
                message: app_error.message().to_string(),
                request_id,
            },
        };

        counter!(
            "api_errors_total",
            "route" => "app",
            "error_code" => app_error.code().to_string()
        )
        .increment(1);

        if matches!(app_error, AppError::AeadIntegrity) {
            counter!("aead_failures_total", "reason" => "integrity").increment(1);
        }

        (status, Json(body)).into_response()
    }
}

fn status_for(err: &AppError) -> StatusCode {
    match err {
        AppError::BadRequest => StatusCode::BAD_REQUEST,
        AppError::Unauthorized => StatusCode::UNAUTHORIZED,
        AppError::Forbidden => StatusCode::FORBIDDEN,
        AppError::NotFound => StatusCode::NOT_FOUND,
        AppError::Conflict => StatusCode::CONFLICT,
        AppError::RateLimited => StatusCode::TOO_MANY_REQUESTS,
        AppError::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        AppError::AeadIntegrity => StatusCode::BAD_REQUEST,
        AppError::ReplayDetected => StatusCode::BAD_REQUEST,
        AppError::ReplayOrSkew => StatusCode::BAD_REQUEST,
        AppError::SignatureInvalid => StatusCode::BAD_REQUEST,
        AppError::EnvelopeRecipientMismatch => StatusCode::BAD_REQUEST,
        AppError::CryptoVersionUnsupported => StatusCode::BAD_REQUEST,
        AppError::DualSignatureRequired => StatusCode::BAD_REQUEST,
    }
}
