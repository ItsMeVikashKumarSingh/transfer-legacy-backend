#[test]
fn crypto_version_error_code_is_stable() {
    assert_eq!(
        transfer_legacy_shared_types::AppError::CryptoVersionUnsupported.code(),
        "ERR_CRYPTO_VERSION_UNSUPPORTED"
    );
    assert_ne!(
        transfer_legacy_shared_types::CURRENT_CRYPTO_VERSION.as_str(),
        "aead-old-v0"
    );
}
