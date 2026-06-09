use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterInitRequest {
    pub user_id: Uuid,
    pub registration_request: String,
    pub credential_identifier: Option<String>,
    pub verification_token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterInitResponse {
    pub session_id: Uuid,
    pub registration_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RegisterFinishRequest {
    pub session_id: Uuid,
    pub registration_upload: String,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
    pub emk_blob: String,
    pub argon2_params: serde_json::Value,
    pub enc_legal_name: String,
    pub enc_email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterFinishResponse {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginInitRequest {
    pub user_id: Uuid,
    pub credential_request: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginInitResponse {
    pub session_id: Uuid,
    pub credential_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginFinishRequest {
    pub session_id: Uuid,
    pub credential_finalization: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LoginFinishResponse {
    pub user_id: Uuid,
    pub session_token: String,
    pub emk_blob: String,
    pub argon2_params: serde_json::Value,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
}
