use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseRecord {
    pub release_id: Uuid,
    pub payload_hash: String,
    pub schema_version: i32,
    pub crypto_version: String,
}
