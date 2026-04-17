use crate::tests::test_utils::{spawn_app, CryptoClient};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use transfer_legacy_shared_types::models::auth::{
    LoginFinishRequest, LoginFinishResponse, LoginInitRequest, LoginInitResponse,
    RegisterFinishRequest, RegisterFinishResponse, RegisterInitRequest, RegisterInitResponse,
};
use uuid::Uuid;

#[tokio::test]
async fn test_full_auth_lifecycle() {
    let ctx = spawn_app().await;
    let mut client = CryptoClient::new(&ctx);

    let user_id = Uuid::new_v4();
    let email = format!("test-{}@example.com", user_id);
    crate::tests::test_utils::create_test_user(&ctx.db, user_id, &email).await;

    let password = "Password123!";

    // --- 1. Registration Init ---
    let (reg_state, reg_req) = crate::tests::test_utils::test_client_register_init(password);
    let reg_init_req = RegisterInitRequest {
        user_id,
        registration_request: reg_req,
        credential_identifier: None,
    };

    let reg_init_res: RegisterInitResponse = client
        .post_aead("/v1/auth/register/init", &reg_init_req)
        .await;

    // --- 2. Registration Finish ---
    let reg_upload = crate::tests::test_utils::test_client_register_finish(
        reg_state,
        &reg_init_res.registration_response,
    );
    let reg_finish_req = RegisterFinishRequest {
        session_id: reg_init_res.session_id,
        registration_upload: reg_upload,
        ed25519_pubkey: URL_SAFE_NO_PAD.encode(b"test-pubkey"),
        x25519_pubkey: URL_SAFE_NO_PAD.encode(b"test-pubkey"),
        kyber768_pubkey: URL_SAFE_NO_PAD.encode(b"test-pubkey"),
        emk_blob: URL_SAFE_NO_PAD.encode(b"test-emk"),
        argon2_params: serde_json::json!({"t": 1, "m": 65536, "p": 4}),
        enc_legal_name: URL_SAFE_NO_PAD.encode(b"test-enc"),
        enc_email: URL_SAFE_NO_PAD.encode(b"test-enc"),
    };

    let reg_finish_res: RegisterFinishResponse = client
        .post_aead("/v1/auth/register/finish", &reg_finish_req)
        .await;
    assert_eq!(reg_finish_res.user_id, user_id);

    // --- 3. Login Init ---
    let (login_state, login_req) = crate::tests::test_utils::test_client_login_init(password);
    let login_init_req = LoginInitRequest {
        user_id,
        credential_request: login_req,
    };

    let login_init_res: LoginInitResponse = client
        .post_aead("/v1/auth/login/init", &login_init_req)
        .await;

    // --- 4. Login Finish ---
    let credential_finalization = crate::tests::test_utils::test_client_login_finish(
        login_state,
        &login_init_res.credential_response,
    );

    let login_finish_req = LoginFinishRequest {
        session_id: login_init_res.session_id,
        credential_finalization,
    };

    let login_finish_res: LoginFinishResponse = client
        .post_aead("/v1/auth/login/finish", &login_finish_req)
        .await;

    // Verify we got a session/token
    assert_eq!(login_finish_res.user_id, user_id);
    assert!(
        !login_finish_res.session_token.is_empty(),
        "Login did not return a session token"
    );
    println!(
        "Successfully verified full OPAQUE Auth flow for {}",
        user_id
    );
}
