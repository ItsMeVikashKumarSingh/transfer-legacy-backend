use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct PolicyUpsertRequest {
    pub owner_id: Uuid,
    pub policy_id: Option<Uuid>,
    pub policy_type: String,
    pub cadence: String,
    pub m_of_n: Option<serde_json::Value>,
    pub beneficiaries: serde_json::Value,
    pub approvers: serde_json::Value,
    pub release_conditions: Option<serde_json::Value>,
    pub stepup_challenge_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PolicyUpsertResponse {
    pub policy_id: Uuid,
    pub pending_at: DateTime<Utc>,
    pub grace_deadline: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct InviteRequest {
    pub email: String,
    pub role: String,
    pub stepup_challenge_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct InviteResponse {
    pub invite_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HeartbeatRequest {
    pub policy_id: Uuid,
    pub device_id: Uuid,
    pub ts: i64,
    pub device_sig: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HeartbeatResponse {
    pub policy_id: Uuid,
    pub pending_at: DateTime<Utc>,
    pub grace_deadline: DateTime<Utc>,
    pub status: String,
}
