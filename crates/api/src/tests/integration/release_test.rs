#[cfg(test)]
mod tests {
    use transfer_legacy_shared_types::models::release_record::ReleaseRecord;
    use uuid::Uuid;

    #[test]
    fn test_release_record_model_construction() {
        let release_id = Uuid::new_v4();
        let payload_hash =
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();

        let record = ReleaseRecord {
            release_id,
            payload_hash: payload_hash.clone(),
            schema_version: 1,
            crypto_version: "hybrid-kyber768-x25519-v1".to_string(),
        };

        assert_eq!(record.release_id, release_id);
        assert_eq!(record.payload_hash, payload_hash);
        assert_eq!(record.schema_version, 1);
        assert_eq!(record.crypto_version, "hybrid-kyber768-x25519-v1");

        let serialized = serde_json::to_string(&record).expect("serialization works");
        let deserialized: ReleaseRecord =
            serde_json::from_str(&serialized).expect("deserialization works");
        assert_eq!(deserialized.release_id, release_id);
    }
}
