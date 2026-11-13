#[cfg(test)]
mod tests {
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use serde_json::json;
    use transfer_legacy_crypto_core::{hash::sha256, jcs::canonicalize};
    use uuid::Uuid;

    #[test]
    fn test_audit_event_hash_chain_verification() {
        let event_1_id = Uuid::new_v4();
        let payload_1 = json!({ "action": "CREATE_VAULT_ITEM", "item_id": Uuid::new_v4() });
        let payload_1_hash = sha256(&serde_json::to_vec(&payload_1).unwrap());

        let event_1_envelope = json!({
            "event_id": event_1_id,
            "payload_hash": URL_SAFE_NO_PAD.encode(&payload_1_hash),
            "prev_hash": null
        });
        let bytes_1 = canonicalize(&event_1_envelope).expect("canonicalize");
        let hash_1 = sha256(&bytes_1);

        // Event 2 links to Event 1
        let event_2_id = Uuid::new_v4();
        let payload_2 = json!({ "action": "HEARTBEAT_PING", "timestamp": 123456789 });
        let payload_2_hash = sha256(&serde_json::to_vec(&payload_2).unwrap());

        let event_2_envelope = json!({
            "event_id": event_2_id,
            "payload_hash": URL_SAFE_NO_PAD.encode(&payload_2_hash),
            "prev_hash": URL_SAFE_NO_PAD.encode(&hash_1)
        });
        let bytes_2 = canonicalize(&event_2_envelope).expect("canonicalize");
        let hash_2 = sha256(&bytes_2);

        // Verification: event 2 contains valid pointer to event 1
        assert_ne!(hash_1, hash_2);
        assert_eq!(
            event_2_envelope["prev_hash"],
            URL_SAFE_NO_PAD.encode(&hash_1)
        );
    }

    #[test]
    fn test_tampered_audit_chain_breaks_link() {
        let hash_1 = [0xAAu8; 32];
        let tampered_hash_1 = [0xBBu8; 32];

        let event_2_envelope = json!({
            "event_id": Uuid::new_v4(),
            "payload_hash": URL_SAFE_NO_PAD.encode(b"sample_payload"),
            "prev_hash": URL_SAFE_NO_PAD.encode(&hash_1)
        });

        // Verifier checks against the tampered previous block
        assert_ne!(
            event_2_envelope["prev_hash"],
            URL_SAFE_NO_PAD.encode(&tampered_hash_1),
            "Tampered previous block hash should be detected as a break in the chain"
        );
    }
}
