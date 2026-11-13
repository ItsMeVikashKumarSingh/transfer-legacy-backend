use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsUser {
    pub id: Uuid,
    pub email: String,
    pub plan: String,
    pub status: String,
    pub vault_assets_count: i64,
    pub guardians_count: i64,
    pub heartbeat_streak_days: i32,
    pub last_active: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserStatusRequest {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateUserPlanRequest {
    pub plan: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsPlan {
    pub id: String,
    pub name: String,
    pub monthly_price: f64,
    pub yearly_price: f64,
    pub description: String,
    pub features: Vec<String>,
    pub max_guardians: i32,
    pub max_assets: i32,
    pub highlighted: bool,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsCreatePlanRequest {
    pub id: String,
    pub name: String,
    pub monthly_price: f64,
    pub yearly_price: f64,
    pub description: String,
    pub features: Vec<String>,
    pub max_guardians: i32,
    pub max_assets: i32,
    pub highlighted: bool,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsUpdatePlanRequest {
    pub name: Option<String>,
    pub monthly_price: Option<f64>,
    pub yearly_price: Option<f64>,
    pub description: Option<String>,
    pub features: Option<Vec<String>>,
    pub max_guardians: Option<i32>,
    pub max_assets: Option<i32>,
    pub highlighted: Option<bool>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsCoupon {
    pub id: Uuid,
    pub code: String,
    pub discount_percent: i32,
    pub max_uses: i32,
    pub current_uses: i32,
    pub expires_at: DateTime<Utc>,
    pub active: bool,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsCreateCouponRequest {
    pub code: String,
    pub discount_percent: i32,
    pub max_uses: i32,
    pub expires_at: DateTime<Utc>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsUpdateCouponRequest {
    pub discount_percent: Option<i32>,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub active: Option<bool>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCouponRequest {
    pub code: String,
    pub amount: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateCouponResponse {
    pub valid: bool,
    pub discount_percent: i32,
    pub discount_amount: f64,
    pub final_amount: f64,
    pub message: String,
}

