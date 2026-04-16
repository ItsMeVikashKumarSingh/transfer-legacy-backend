use anyhow::Result;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use transfer_legacy_crypto_core::opaque::{create_server_setup, server_setup_to_b64};

fn main() -> Result<()> {
    println!("Generating production secret seeds...");

    // 1. OPAQUE Server Setup
    let setup = create_server_setup();
    let opaque_setup = server_setup_to_b64(&setup);
    println!("OPAQUE_SERVER_SETUP_B64={}", opaque_setup);

    // 2. SERVER_AEAD_KEY
    let mut aead_key = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut aead_key);
    println!("SERVER_AEAD_KEY_B64={}", URL_SAFE_NO_PAD.encode(aead_key));

    // 3. SERVER_HMAC_SECRET
    let mut hmac_secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut hmac_secret);
    println!("SERVER_HMAC_SECRET={}", URL_SAFE_NO_PAD.encode(hmac_secret));

    // 4. JWT_SECRET
    let mut jwt_secret = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut jwt_secret);
    println!("JWT_SECRET={}", URL_SAFE_NO_PAD.encode(jwt_secret));

    Ok(())
}
