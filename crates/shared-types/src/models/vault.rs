use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateItemRequest {
    pub user_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub crypto_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateItemResponse {
    pub item_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListItemsRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ItemSummary {
    pub item_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListItemsResponse {
    pub items: Vec<ItemSummary>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetItemRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetItemResponse {
    pub item_id: Uuid,
    pub ciphertext: String,
    pub item_meta: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteItemRequest {
    pub user_id: Uuid,
    pub item_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeleteItemResponse {
    pub status: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct CreateShareRequest {
    pub owner_id: Uuid,
    pub item_id: Uuid,
    pub grantee_id: Uuid,
    pub envelope: serde_json::Value,
    pub grant_sig: String,
    pub crypto_version: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateShareResponse {
    pub share_id: Uuid,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ListSharesRequest {
    pub owner_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShareSummary {
    pub share_id: Uuid,
    pub item_id: Uuid,
    pub grantee_id: Uuid,
    pub envelope: serde_json::Value,
    pub grant_sig: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListSharesResponse {
    pub shares: Vec<ShareSummary>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RevokeShareRequest {
    pub owner_id: Uuid,
    pub share_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RevokeShareResponse {
    pub status: String,
}
