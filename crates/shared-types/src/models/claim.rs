use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaimInitiateRequest {
    pub policy_id: Uuid,
    pub claimant_person_id: Uuid,
    pub claim_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimInitiateResponse {
    pub claim_id: Uuid,
    pub confirmation_deadline: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ClaimConfirmRequest {
    pub claim_id: Uuid,
    pub claimant_person_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClaimConfirmResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PresignAttachmentRequest {
    pub claim_id: Uuid,
    pub content_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PresignAttachmentResponse {
    pub attachment_id: Uuid,
    pub upload_url: String,
    pub object_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConfirmAttachmentRequest {
    pub attachment_id: Uuid,
    pub sha256_b64: String,
    pub size_bytes: i64,
    pub mime_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfirmAttachmentResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AttestationRequest {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
    pub approver_person_id: Uuid,
    pub statement: serde_json::Value,
    pub signature_b64: String,
    pub public_key_b64: String,
    pub signature_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AttestationResponse {
    pub attestation_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReleaseRecordRequest {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
    pub payload: serde_json::Value,
    pub schema_version: i32,
    pub crypto_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ReleaseRecordResponse {
    pub release_id: Uuid,
    pub signature: String,
}
