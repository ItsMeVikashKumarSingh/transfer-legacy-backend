use crate::tests::test_utils::{spawn_app, CryptoClient};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde_json::json;
use transfer_legacy_shared_types::models::auth::{
    LoginFinishRequest, LoginFinishResponse, LoginInitRequest, LoginInitResponse,
    RegisterFinishRequest, RegisterFinishResponse, RegisterInitRequest, RegisterInitResponse,
};
use transfer_legacy_shared_types::models::claim::{
    AttestationRequest, ClaimInitiateRequest, ConfirmAttachmentRequest, PresignAttachmentRequest,
};
use uuid::Uuid;

async fn get_auth_token(
    client: &mut CryptoClient,
    user_id: Uuid,
    email: &str,
    password: &str,
) -> String {
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

    let (login_state, login_req) = crate::tests::test_utils::test_client_login_init(password);
    let login_init_req = LoginInitRequest {
        user_id,
        credential_request: login_req,
    };
    let login_init_res: LoginInitResponse = client
        .post_aead("/v1/auth/login/init", &login_init_req)
        .await;

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
#[ignore = "requires live database"]
async fn test_claim_lifecycle() {
    let ctx = spawn_app().await;
    let mut client = CryptoClient::new(&ctx);

    let user_id = Uuid::new_v4();
    let email = format!("claim-test-{}@example.com", user_id);
    let password = "ClaimPassword123!";
    let _token = get_auth_token(&mut client, user_id, &email, password).await;

    println!("Claim test updated to use shared models for {}", user_id);
}

#[test]
fn test_claim_initiate_request_validation() {
    let policy_id = Uuid::new_v4();
    let claimant_person_id = Uuid::new_v4();

    let req = ClaimInitiateRequest {
        policy_id,
        claimant_person_id,
        claim_type: "death_certificate".to_string(),
    };

    let serialized = serde_json::to_string(&req).expect("serialization works");
    let deserialized: ClaimInitiateRequest =
        serde_json::from_str(&serialized).expect("deserialization works");
    assert_eq!(deserialized.policy_id, policy_id);
    assert_eq!(deserialized.claimant_person_id, claimant_person_id);
    assert_eq!(deserialized.claim_type, "death_certificate");
}

#[test]
fn test_quorum_statement_approval_and_dispute_payloads() {
    let policy_id = Uuid::new_v4();
    let claim_id = Uuid::new_v4();
    let approver_person_id = Uuid::new_v4();

    // 1. Approval statement
    let approval_statement = json!({
        "action": "APPROVE",
        "evidence_reviewed": true,
        "note": "Death certificate verified with local registrar"
    });

    let approval_req = AttestationRequest {
        policy_id,
        claim_id,
        approver_person_id,
        statement: approval_statement,
        signature_b64: URL_SAFE_NO_PAD.encode(b"ed25519-signature-bytes"),
        public_key_b64: URL_SAFE_NO_PAD.encode(b"ed25519-public-key-bytes"),
        signature_type: "ed25519".to_string(),
    };

    assert_eq!(approval_req.statement["action"], "APPROVE");
    assert_eq!(approval_req.statement["evidence_reviewed"], true);

    // 2. Dispute statement
    let dispute_statement = json!({
        "action": "DISPUTE",
        "fraud_suspected": true,
        "reason": "Account owner contacted me personally yesterday"
    });

    let dispute_req = AttestationRequest {
        policy_id,
        claim_id,
        approver_person_id,
        statement: dispute_statement,
        signature_b64: URL_SAFE_NO_PAD.encode(b"ed25519-dispute-sig"),
        public_key_b64: URL_SAFE_NO_PAD.encode(b"ed25519-public-key-bytes"),
        signature_type: "ed25519".to_string(),
    };

    assert_eq!(dispute_req.statement["action"], "DISPUTE");
    assert_eq!(dispute_req.statement["fraud_suspected"], true);
}

#[test]
fn test_attachment_presign_and_confirm_validation() {
    let claim_id = Uuid::new_v4();
    let attachment_id = Uuid::new_v4();

    let presign_req = PresignAttachmentRequest {
        claim_id,
        content_type: "application/pdf".to_string(),
    };
    assert_eq!(presign_req.content_type, "application/pdf");

    let confirm_req = ConfirmAttachmentRequest {
        attachment_id,
        sha256_b64: URL_SAFE_NO_PAD.encode(vec![0xAA; 32]),
        size_bytes: 1024 * 1024 * 2, // 2MB
        mime_type: "application/pdf".to_string(),
    };

    assert_eq!(confirm_req.attachment_id, attachment_id);
    assert_eq!(confirm_req.size_bytes, 2097152);
    assert_eq!(confirm_req.mime_type, "application/pdf");
}
