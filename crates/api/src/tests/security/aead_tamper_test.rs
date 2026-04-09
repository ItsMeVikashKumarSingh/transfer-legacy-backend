#[test]
fn aead_tamper_is_rejected() {
    use transfer_legacy_crypto_core::aead::{decrypt, encrypt};

    let key = [7u8; 32];
    let aad = b"req-123|12|1710000000";
    let plaintext = br#"{"ok":true}"#;
    let envelope = encrypt(&key, plaintext, aad).expect("encrypt");
    let mut tampered = envelope.ciphertext.clone();
    tampered[0] ^= 0x80;

    let err = decrypt(&key, &envelope.nonce, &tampered, aad).expect_err("tamper must fail");
    assert!(matches!(err, transfer_legacy_crypto_core::aead::AeadError::Decrypt));
}
