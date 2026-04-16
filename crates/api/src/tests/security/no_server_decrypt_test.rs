use walkdir::WalkDir;

#[test]
fn no_server_decrypt_calls_in_api_or_worker() {
    let mut violations = Vec::new();
    for root in ["crates/api/src", "crates/worker/src"] {
        for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            if path
                .components()
                .any(|component| component.as_os_str() == "tests")
            {
                continue;
            }
            if path.ends_with("aead_transport.rs") {
                continue;
            }
            let src = std::fs::read_to_string(path).unwrap_or_default();
            if src.contains("::aead::decrypt(") || src.contains(" aead::decrypt(") {
                violations.push(path.display().to_string());
            }
        }
    }
    assert!(
        violations.is_empty(),
        "decrypt usage forbidden in api/worker: {:?}",
        violations
    );
}
