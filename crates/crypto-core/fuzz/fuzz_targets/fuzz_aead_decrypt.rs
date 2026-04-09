#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use transfer_legacy_crypto_core::aead::decrypt;

#[derive(Arbitrary, Debug)]
struct FuzzInput {
    key: Vec<u8>,
    nonce: Vec<u8>,
    ciphertext: Vec<u8>,
    aad: Vec<u8>,
}

fuzz_target!(|input: FuzzInput| {
    let _ = decrypt(&input.key, &input.nonce, &input.ciphertext, &input.aad);
});
