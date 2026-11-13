use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use transfer_legacy_shared_types::AppError;
use uuid::Uuid;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("jwt encode error")]
    Encode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionClaims {
    pub sub: String,
    pub exp: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

pub fn issue_session_token(config: &Config, user_id: Uuid) -> Result<String, SessionError> {
    issue_session_token_with_secret(&config.jwt_secret, user_id)
}

pub fn verify_session_token(config: &Config, token: &str) -> Result<AuthenticatedUser, AppError> {
    verify_session_token_with_secret(&config.jwt_secret, token)
}

pub fn issue_session_token_with_secret(jwt_secret: &str, user_id: Uuid) -> Result<String, SessionError> {
    let exp = OffsetDateTime::now_utc().unix_timestamp() + 3600;
    let claims = SessionClaims {
        sub: user_id.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .map_err(|_| SessionError::Encode)
}

pub fn verify_session_token_with_secret(jwt_secret: &str, token: &str) -> Result<AuthenticatedUser, AppError> {
    let validation = Validation::default();
    let token_data = decode::<SessionClaims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &validation,
    )
    .map_err(|_| AppError::Unauthorized)?;

    let user_id = Uuid::parse_str(&token_data.claims.sub).map_err(|_| AppError::Unauthorized)?;
    Ok(AuthenticatedUser { user_id })
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-jwt-secret-with-sufficient-length-32-bytes!!";

    #[test]
    fn test_session_token_issue_and_verify_success() {
        let user_id = Uuid::new_v4();

        let token = issue_session_token_with_secret(TEST_SECRET, user_id).expect("issue token");
        let verified = verify_session_token_with_secret(TEST_SECRET, &token).expect("verify token");

        assert_eq!(verified.user_id, user_id);
    }

    #[test]
    fn test_session_token_tampered_fails() {
        let user_id = Uuid::new_v4();

        let token = issue_session_token_with_secret(TEST_SECRET, user_id).expect("issue token");
        let tampered = format!("{}tampered", token);

        let err = verify_session_token_with_secret(TEST_SECRET, &tampered).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }

    #[test]
    fn test_session_token_wrong_secret_fails() {
        let secret_a = "secret-one-123456789012345678901234";
        let secret_b = "secret-two-123456789012345678901234";

        let user_id = Uuid::new_v4();
        let token = issue_session_token_with_secret(secret_a, user_id).expect("issue token");

        let err = verify_session_token_with_secret(secret_b, &token).unwrap_err();
        assert!(matches!(err, AppError::Unauthorized));
    }
}
