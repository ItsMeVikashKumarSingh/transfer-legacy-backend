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
    pub waitlist_enabled: bool,
    pub theme_config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactConfig {
    pub office_address: Option<String>,
    pub map_embed_url: Option<String>,
    pub emails: serde_json::Value,
    pub phones: serde_json::Value,
    pub social_links: serde_json::Value,
    pub working_hours: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactMessage {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub subject: Option<String>,
    pub message: String,
    pub metadata: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
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
