#[cfg(test)]
mod tests {
    use crate::hybrid_kem::{HybridEnvelope, HybridEnvelopeMeta};

    fn make_valid_envelope() -> HybridEnvelope {
        HybridEnvelope {
            schema_version: 1,
            wrapper: "hybrid-wrap-v1".to_string(),
            owner_id: "user-123".to_string(),
            item_id: "item-456".to_string(),
            eph_x25519_pub: "x25519-mock-pubkey".to_string(),
            kyber_ct: "kyber-mock-ciphertext".to_string(),
            nonce: "nonce-mock-24bytes".to_string(),
            payload: "encrypted-payload-blob".to_string(),
            meta: HybridEnvelopeMeta {
                owner_id: "user-123".to_string(),
                item_id: "item-456".to_string(),
                crypto_version: "2".to_string(),
            },
            grant_sig: None,
        }
    }

    #[test]
    fn test_valid_envelope_binding_passes() {
        let envelope = make_valid_envelope();
        assert!(envelope.validate_binding("user-123", "item-456").is_ok());
    }

    #[test]
    fn test_invalid_wrapper_type_rejected() {
        let mut envelope = make_valid_envelope();
        envelope.wrapper = "legacy-v0".to_string();
        let result = envelope.validate_binding("user-123", "item-456");
        assert_eq!(
            result.unwrap_err(),
            "invalid wrapper type; expected hybrid-wrap-v1"
        );
    }

    #[test]
    fn test_owner_mismatch_rejected() {
        let envelope = make_valid_envelope();
        let result = envelope.validate_binding("imposter-user", "item-456");
        assert_eq!(
            result.unwrap_err(),
            "owner_id mismatch in envelope metadata"
        );
    }

    #[test]
    fn test_item_mismatch_rejected() {
        let envelope = make_valid_envelope();
        let result = envelope.validate_binding("user-123", "unauthorized-item");
        assert_eq!(result.unwrap_err(), "item_id mismatch in envelope metadata");
    }
}
