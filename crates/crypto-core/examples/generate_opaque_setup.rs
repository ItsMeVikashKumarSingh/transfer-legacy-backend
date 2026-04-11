use transfer_legacy_crypto_core::opaque::{create_server_setup, server_setup_to_b64};

fn main() {
    let setup = create_server_setup();
    let encoded = server_setup_to_b64(&setup);
    println!("{}", encoded);
}
