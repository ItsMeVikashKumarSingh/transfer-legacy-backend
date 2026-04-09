#[test]
fn skew_error_code_is_stable() {
    assert_eq!(
        transfer_legacy_shared_types::AppError::ReplayOrSkew.code(),
        "ERR_REPLAY_OR_SKEW"
    );
}
