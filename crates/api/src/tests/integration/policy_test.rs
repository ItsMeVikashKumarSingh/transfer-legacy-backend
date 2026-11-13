#[cfg(test)]
mod tests {
    use serde_json::json;
    use transfer_legacy_shared_types::models::inheritance::{
        HeartbeatRequest, PolicyUpsertRequest,
    };
    use transfer_legacy_shared_types::models::policy::PolicyStatus;
    use uuid::Uuid;

    #[test]
    fn test_policy_status_state_transitions() {
        let statuses = vec![
            PolicyStatus::Active,
            PolicyStatus::Pending,
            PolicyStatus::Investigating,
            PolicyStatus::ReleaseReady,
            PolicyStatus::ConflictPending,
            PolicyStatus::ManualReview,
            PolicyStatus::Released,
            PolicyStatus::Cancelled,
        ];

        for status in statuses {
            let serialized = serde_json::to_string(&status).expect("serialization works");
            let deserialized: PolicyStatus =
                serde_json::from_str(&serialized).expect("deserialization works");
            assert_eq!(status, deserialized);
        }
    }

    #[test]
    fn test_policy_upsert_request_schema_validation() {
        let owner_id = Uuid::new_v4();
        let stepup_id = Uuid::new_v4();

        let req = PolicyUpsertRequest {
            owner_id,
            policy_id: None,
            policy_type: "dead_man_switch".to_string(),
            cadence: "90d".to_string(),
            m_of_n: Some(json!({ "threshold": 2, "total": 3 })),
            beneficiaries: json!([
                { "id": Uuid::new_v4(), "share_percent": 100 }
            ]),
            approvers: json!([
                { "id": Uuid::new_v4(), "role": "guardian" },
                { "id": Uuid::new_v4(), "role": "guardian" }
            ]),
            release_conditions: Some(json!({ "require_death_certificate": true })),
            stepup_challenge_id: stepup_id,
        };

        let json_value = serde_json::to_value(&req).expect("valid serialization");
        assert_eq!(json_value["cadence"], "90d");
        assert_eq!(json_value["policy_type"], "dead_man_switch");
        assert_eq!(json_value["owner_id"], owner_id.to_string());
    }

    #[test]
    fn test_heartbeat_request_validation() {
        let policy_id = Uuid::new_v4();
        let device_id = Uuid::new_v4();
        let now = chrono::Utc::now().timestamp();

        let req = HeartbeatRequest {
            policy_id,
            device_id,
            ts: now,
            device_sig: "base64-mock-signature".to_string(),
        };

        let serialized = serde_json::to_string(&req).expect("serialize heartbeat");
        let parsed: HeartbeatRequest = serde_json::from_str(&serialized).expect("deserialize");
        assert_eq!(parsed.policy_id, policy_id);
        assert_eq!(parsed.device_id, device_id);
        assert_eq!(parsed.ts, now);
    }
}
