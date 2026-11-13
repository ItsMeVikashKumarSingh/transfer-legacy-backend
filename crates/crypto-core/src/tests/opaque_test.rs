#[cfg(test)]
mod tests {
    use crate::opaque::{create_server_setup, server_setup_from_b64, server_setup_to_b64};

    #[test]
    fn test_opaque_server_setup_b64_roundtrip() {
        let setup = create_server_setup();
        let encoded = server_setup_to_b64(&setup);
        assert!(!encoded.is_empty());

        let decoded = server_setup_from_b64(&encoded).expect("deserialization succeeds");
        let re_encoded = server_setup_to_b64(&decoded);
        assert_eq!(encoded, re_encoded);
    }
}
