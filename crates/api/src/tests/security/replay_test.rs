#[test]
fn replay_error_code_is_stable() {
    assert_eq!(
        transfer_legacy_shared_types::AppError::ReplayDetected.code(),
        "ERR_REPLAY_DETECTED"
    );
}
