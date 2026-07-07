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
    OtpExpired,
    OtpInvalid,
    UserNotFound,
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
            AppError::OtpExpired => "ERR_OTP_EXPIRED",
            AppError::OtpInvalid => "ERR_OTP_INVALID",
            AppError::UserNotFound => "ERR_USER_NOT_FOUND",
        }
    }

    pub fn message(&self) -> &'static str {
        match self {
            AppError::BadRequest => "The request is invalid or could not be understood.",
            AppError::Unauthorized => "Authentication is failed or required.",
            AppError::Forbidden => "Access denied. You do not have permission to perform this action.",
            AppError::NotFound => "The requested resource could not be found.",
            AppError::Conflict => "A resource conflict occurred (e.g. data already exists).",
            AppError::RateLimited => "Too many requests. Please wait and try again later.",
            AppError::Internal => "An unexpected server error occurred. Please try again later.",
            AppError::AeadIntegrity => "Request data integrity check failed.",
            AppError::ReplayDetected => "Request rejected: potential replay attack detected.",
            AppError::ReplayOrSkew => "Request timestamp is invalid or has expired.",
            AppError::SignatureInvalid => "Cryptographic signature verification failed.",
            AppError::EnvelopeRecipientMismatch => "Recipient identifier does not match the envelope destination.",
            AppError::CryptoVersionUnsupported => "The requested cryptographic protocol version is not supported.",
            AppError::DualSignatureRequired => "This action requires dual operator approval signatures.",
            AppError::OtpExpired => "Verification code expired or not found. Please request a new code.",
            AppError::OtpInvalid => "Incorrect verification code. Please check and try again.",
            AppError::UserNotFound => "User does not exist.",
        }
    }
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl std::error::Error for AppError {}
