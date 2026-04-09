#[test]
fn forged_signature_is_rejected() {
    use transfer_legacy_crypto_core::signatures::verify_ed25519;

    let pub_key = [0u8; 32];
    let message = b"release-record-digest";
    let signature = [0u8; 64];

    let err = verify_ed25519(&pub_key, message, &signature).expect_err("forged signature must fail");
    assert!(matches!(
        err,
        transfer_legacy_crypto_core::signatures::SignatureError::InvalidKey
            | transfer_legacy_crypto_core::signatures::SignatureError::InvalidSignature
    ));
}
