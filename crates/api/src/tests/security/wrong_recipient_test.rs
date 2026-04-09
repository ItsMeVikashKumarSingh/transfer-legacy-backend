#[test]
fn wrong_recipient_error_code_is_stable() {
    assert_eq!(
        transfer_legacy_shared_types::AppError::EnvelopeRecipientMismatch.code(),
        "ERR_ENVELOPE_RECIPIENT_MISMATCH"
    );
}
