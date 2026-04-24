use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrandingConfig {
    pub brand_name: String,
    pub logo_url: Option<String>,
    pub support_email: Option<String>,
    pub support_phone: Option<String>,
    pub support_address: Option<String>,
    pub theme_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppContent {
    pub slug: String,
    pub body: serde_json::Value,
    pub version: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistSignupRequest {
    pub email: String,
    pub name: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaitlistEntry {
    pub id: Uuid,
    pub email: String,
    pub name: Option<String>,
    pub meta: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}
