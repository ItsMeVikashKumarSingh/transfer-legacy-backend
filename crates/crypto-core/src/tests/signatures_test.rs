#[cfg(test)]
mod tests {
    use crate::signatures::verify_ed25519;
    use ed25519_dalek::Signer;
    use rand::rngs::OsRng;
    use rand::RngCore;

    #[test]
    fn test_signature_verification_succeeds_for_valid_pair() {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        let message = b"Transfer Legacy Audit Log Record #1001";
        let signature = signing_key.sign(message);

        let result = verify_ed25519(verifying_key.as_bytes(), message, &signature.to_bytes());
        assert!(result.is_ok());
    }

    #[test]
    fn test_signature_verification_fails_for_altered_message() {
        let mut csprng = OsRng;
        let mut secret_bytes = [0u8; 32];
        csprng.fill_bytes(&mut secret_bytes);
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&secret_bytes);
        let verifying_key = signing_key.verifying_key();

        let message = b"Legitimate transaction payload";
        let signature = signing_key.sign(message);

        let altered_message = b"Tampered transaction payload";
        let result = verify_ed25519(
            verifying_key.as_bytes(),
            altered_message,
            &signature.to_bytes(),
        );
        assert!(result.is_err());
    }
}
