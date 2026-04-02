use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyStatus {
    Active,
    Pending,
    Investigating,
    ReleaseReady,
    ConflictPending,
    ManualReview,
    Released,
    Cancelled,
}
