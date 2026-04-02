use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AppError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    RateLimited,
    Internal,
    AeadIntegrity,
    ReplayDetected,
    ReplayOrSkew,
    SignatureInvalid,
    EnvelopeRecipientMismatch,
    CryptoVersionUnsupported,
    DualSignatureRequired,
}

impl AppError {
    pub fn code(&self) -> &'static str {
        match self {
            AppError::BadRequest => "ERR_BAD_REQUEST",
            AppError::Unauthorized => "ERR_UNAUTHORIZED",
            AppError::Forbidden => "ERR_FORBIDDEN",
            AppError::NotFound => "ERR_NOT_FOUND",
            AppError::Conflict => "ERR_CONFLICT",
            AppError::RateLimited => "ERR_RATE_LIMITED",
            AppError::Internal => "ERR_INTERNAL",
            AppError::AeadIntegrity => "ERR_AEAD_INTEGRITY",
            AppError::ReplayDetected => "ERR_REPLAY_DETECTED",
            AppError::ReplayOrSkew => "ERR_REPLAY_OR_SKEW",
            AppError::SignatureInvalid => "ERR_SIGNATURE_INVALID",
            AppError::EnvelopeRecipientMismatch => "ERR_ENVELOPE_RECIPIENT_MISMATCH",
            AppError::CryptoVersionUnsupported => "ERR_CRYPTO_VERSION_UNSUPPORTED",
            AppError::DualSignatureRequired => "ERR_DUAL_SIGNATURE_REQUIRED",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            AppError::BadRequest => "Invalid request.",
            AppError::Unauthorized => "Authentication required.",
            AppError::Forbidden => "Forbidden.",
            AppError::NotFound => "Resource not found.",
            AppError::Conflict => "Conflict.",
            AppError::RateLimited => "Too many requests.",
            AppError::Internal => "Internal server error.",
            AppError::AeadIntegrity => "Request integrity check failed.",
            AppError::ReplayDetected => "Replay detected.",
            AppError::ReplayOrSkew => "Replay or clock skew detected.",
            AppError::SignatureInvalid => "Signature verification failed.",
            AppError::EnvelopeRecipientMismatch => "Envelope recipient mismatch.",
            AppError::CryptoVersionUnsupported => "Unsupported crypto version.",
            AppError::DualSignatureRequired => "Dual operator signature required.",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl std::error::Error for AppError {}
