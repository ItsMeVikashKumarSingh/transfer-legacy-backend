use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use uuid::Uuid;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum SessionError {
    #[error("jwt encode error")]
    Encode,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: i64,
}

pub fn issue_session_token(config: &Config, user_id: Uuid) -> Result<String, SessionError> {
    let exp = OffsetDateTime::now_utc().unix_timestamp() + 3600;
    let claims = Claims {
        sub: user_id.to_string(),
        exp,
    };
    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(config.jwt_secret.as_bytes()),
    )
    .map_err(|_| SessionError::Encode)
}
