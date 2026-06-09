use crate::tests::test_utils::{spawn_app, CryptoClient};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use transfer_legacy_shared_types::models::auth::{
    LoginFinishRequest, LoginFinishResponse, LoginInitRequest, LoginInitResponse,
    RegisterFinishRequest, RegisterFinishResponse, RegisterInitRequest, RegisterInitResponse,
};
use transfer_legacy_shared_types::models::vault::{
    CreateItemRequest, CreateItemResponse, DeleteItemResponse, GetItemResponse, ItemSummary,
    ListItemsResponse,
};
use uuid::Uuid;

async fn get_auth_token(client: &mut CryptoClient, user_id: Uuid, email: &str, password: &str) -> String {
    // 1. Registration Init
    let (reg_state, reg_req) = crate::tests::test_utils::test_client_register_init(password);
    let reg_init_req = RegisterInitRequest {
        user_id,
        registration_request: reg_req,
        credential_identifier: Some(email.to_string()),
        verification_token: "test-bypass-token".to_string(),
    };
    let reg_init_res: RegisterInitResponse = client
        .post_aead("/v1/auth/register/init", &reg_init_req)
        .await;

    // 2. Registration Finish
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
    let _: RegisterFinishResponse = client
        .post_aead("/v1/auth/register/finish", &reg_finish_req)
        .await;

    // 3. Login Init
    let (login_state, login_req) = crate::tests::test_utils::test_client_login_init(password);
    let login_init_req = LoginInitRequest {
        user_id,
        credential_request: login_req,
    };
    let login_init_res: LoginInitResponse = client
        .post_aead("/v1/auth/login/init", &login_init_req)
        .await;

    // 4. Login Finish
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

    client.set_token(login_finish_res.session_token.clone());
    login_finish_res.session_token
}

#[tokio::test]
async fn test_vault_lifecycle() {
    let ctx = spawn_app().await;
    let mut client = CryptoClient::new(&ctx);

    let user_id = Uuid::new_v4();
    let email = format!("vault-test-{}@example.com", user_id);
    let password = "VaultPassword123!";
    let _token = get_auth_token(&mut client, user_id, &email, password).await;

    // --- 1. Create Vault Item ---
    let ciphertext_raw = b"secret data";
    let create_req = CreateItemRequest {
        user_id,
        ciphertext: URL_SAFE_NO_PAD.encode(ciphertext_raw),
        item_meta: Some(serde_json::json!({"title": "Test Item"})),
        crypto_version: "v1".to_string(),
    };

    let created_item: CreateItemResponse = client.post_aead("/v1/vault/items", &create_req).await;
    let item_id = created_item.item_id;

    // --- 2. List Vault Items ---
    let list_res: ListItemsResponse = client
        .post_aead(
            "/v1/vault/items/list",
            &serde_json::json!({"user_id": user_id}),
        )
        .await;
    assert!(!list_res.items.is_empty(), "Vault list should not be empty");
    assert!(
        list_res.items.iter().any(|i| i.item_id == item_id),
        "Created item missing from list"
    );

    // --- 3. Get Specific Item ---
    let fetched_item: GetItemResponse = client
        .post_aead(
            "/v1/vault/items/get",
            &serde_json::json!({"user_id": user_id, "item_id": item_id}),
        )
        .await;
    assert_eq!(fetched_item.item_id, item_id);
    assert_eq!(fetched_item.ciphertext, create_req.ciphertext);

    // --- 4. Delete Item ---
    let delete_res: DeleteItemResponse = client
        .post_aead(
            "/v1/vault/items/delete",
            &serde_json::json!({"user_id": user_id, "item_id": item_id}),
        )
        .await;
    assert_eq!(delete_res.status, "ok");

    println!("Successfully verified Vault lifecycle for {}", user_id);
}
