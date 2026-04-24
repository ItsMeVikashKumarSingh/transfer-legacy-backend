use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsRole {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsAdmin {
    pub id: Uuid,
    pub email: String,
    pub role_id: Uuid,
    pub role_name: String,
    pub is_active: bool,
    pub last_login: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsLoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsLoginResponse {
    pub token: String,
    pub admin: OpsAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsCreateAdminRequest {
    pub email: String,
    pub password: String,
    pub role_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsUpdateAdminRequest {
    pub email: Option<String>,
    pub role_id: Option<Uuid>,
    pub is_active: Option<bool>,
}
