#[cfg(test)]
mod tests {
    use crate::aead::{decrypt, encrypt, AeadError};

    #[test]
    fn test_aead_encrypt_decrypt_roundtrip() {
        let key = [0x42u8; 32];
        let plaintext = b"Zero-Knowledge Transfer Legacy Secret Payload";
        let aad = b"additional-authenticated-data-v1";

        let envelope = encrypt(&key, plaintext, aad).expect("encryption should succeed");
        assert_eq!(envelope.nonce.len(), 24);

        let decrypted = decrypt(&key, &envelope.nonce, &envelope.ciphertext, aad)
            .expect("decryption should succeed");
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_aead_tampered_ciphertext_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"Sensitive Master Key Share";
        let aad = b"aad-metadata";

        let mut envelope = encrypt(&key, plaintext, aad).expect("encryption should succeed");

        // Tamper with one byte of ciphertext
        if let Some(first_byte) = envelope.ciphertext.first_mut() {
            *first_byte ^= 0xFF;
        }

        let result = decrypt(&key, &envelope.nonce, &envelope.ciphertext, aad);
        assert!(matches!(result, Err(AeadError::Decrypt)));
    }

    #[test]
    fn test_aead_tampered_aad_fails() {
        let key = [0x42u8; 32];
        let plaintext = b"Sensitive Vault Secret";
        let aad = b"correct-aad";

        let envelope = encrypt(&key, plaintext, aad).expect("encryption should succeed");

        let tampered_aad = b"forged-aad";
        let result = decrypt(&key, &envelope.nonce, &envelope.ciphertext, tampered_aad);
        assert!(matches!(result, Err(AeadError::Decrypt)));
    }

    #[test]
    fn test_aead_wrong_key_fails() {
        let key_a = [0x42u8; 32];
        let key_b = [0x99u8; 32];
        let plaintext = b"Confidential Secret";
        let aad = b"aad";

        let envelope = encrypt(&key_a, plaintext, aad).expect("encryption should succeed");
        let result = decrypt(&key_b, &envelope.nonce, &envelope.ciphertext, aad);
        assert!(matches!(result, Err(AeadError::Decrypt)));
    }

    #[test]
    fn test_aead_invalid_key_length() {
        let short_key = [0x42u8; 16];
        let plaintext = b"payload";
        let aad = b"aad";

        let result = encrypt(&short_key, plaintext, aad);
        assert!(matches!(result, Err(AeadError::InvalidKey)));
    }
}
