#![no_main]

use libfuzzer_sys::fuzz_target;
use serde_json::Value;
use transfer_legacy_crypto_core::jcs::canonicalize;

fuzz_target!(|data: &[u8]| {
    if let Ok(value) = serde_json::from_slice::<Value>(data) {
        let _ = canonicalize(&value);
    }
});
