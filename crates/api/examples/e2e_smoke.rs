use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use ed25519_dalek::{Signer, SigningKey};
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientRegistration,
    ClientRegistrationFinishParameters,
};
use rand::RngCore;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use transfer_legacy_crypto_core::aead;
use transfer_legacy_crypto_core::opaque::DefaultSuite;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
struct SuccessEnvelope<T> {
    data: T,
    request_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AeadEnvelope {
    nonce: String,
    ciphertext: String,
}

#[derive(Debug, Deserialize)]
struct RegisterInitResponse {
    session_id: Uuid,
    registration_response: String,
    server_nonce: String,
}

#[derive(Debug, Deserialize)]
struct RegisterFinishResponse {
    pub user_id: Uuid,
    pub person_id: Uuid,
}

#[derive(Debug, Deserialize)]
struct LoginInitResponse {
    session_id: Uuid,
    credential_response: String,
    server_nonce: String,
}

#[derive(Debug, Deserialize)]
struct LoginFinishResponse {
    pub user_id: Uuid,
    pub person_id: Uuid,
    pub session_token: String,
    pub emk_blob: String,
    pub argon2_params: Value,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
}

#[derive(Debug, Serialize)]
struct RegisterInitRequest<'a> {
    user_id: Uuid,
    registration_request: &'a str,
    credential_identifier: Option<String>,
}

#[derive(Debug, Serialize)]
struct RegisterFinishRequest {
    session_id: Uuid,
    registration_upload: String,
    ed25519_pubkey: String,
    x25519_pubkey: String,
    kyber768_pubkey: String,
    emk_blob: String,
    argon2_params: Value,
    enc_legal_name: String,
    enc_email: String,
}

#[derive(Debug, Serialize)]
struct LoginInitRequest<'a> {
    user_id: Uuid,
    credential_request: &'a str,
}

#[derive(Debug, Serialize)]
struct LoginFinishRequest<'a> {
    session_id: Uuid,
    credential_finalization: &'a str,
}

#[derive(Debug, Serialize)]
struct DeviceRegisterRequest {
    device_id: Uuid,
    user_id: Uuid,
    ts: i64,
    device_sig: String,
    ed25519_pubkey: String,
    device_meta: Option<Value>,
}

#[derive(Debug, Serialize)]
struct DeviceListRequest {
    user_id: Uuid,
}

fn now_ts() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn aad(req_id: &str, seq: u64, ts: i64) -> Vec<u8> {
    format!("{req_id}|{seq}|{ts}").into_bytes()
}

fn encrypt_aead(key: &[u8], req_id: &str, seq: u64, ts: i64, plaintext: &[u8]) -> AeadEnvelope {
    let env = aead::encrypt(key, plaintext, &aad(req_id, seq, ts)).expect("encrypt");
    AeadEnvelope {
        nonce: URL_SAFE_NO_PAD.encode(env.nonce),
        ciphertext: URL_SAFE_NO_PAD.encode(env.ciphertext),
    }
}

fn decrypt_aead<T: for<'de> Deserialize<'de>>(
    key: &[u8],
    req_id: &str,
    seq: u64,
    ts: i64,
    env: &AeadEnvelope,
) -> T {
    let nonce = URL_SAFE_NO_PAD
        .decode(env.nonce.as_bytes())
        .expect("nonce b64");
    let ciphertext = URL_SAFE_NO_PAD
        .decode(env.ciphertext.as_bytes())
        .expect("cipher b64");
    let pt = aead::decrypt(key, &nonce, &ciphertext, &aad(req_id, seq, ts)).expect("decrypt");
    serde_json::from_slice(&pt).expect("json decode")
}

fn env_from_dotenv_local() -> HashMap<String, String> {
    // Keep parsing intentionally tiny; do not print values.
    let mut map = HashMap::new();
    let content = fs::read_to_string(".env.local").unwrap_or_default();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            map.insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    map
}

#[tokio::main]
async fn main() {
    let env = env_from_dotenv_local();
    let base_url = std::env::var("TL_BASE_URL")
        .ok()
        .unwrap_or_else(|| "http://127.0.0.1:8080".to_string());
    let server_aead_key_b64 = std::env::var("SERVER_AEAD_KEY")
        .ok()
        .or_else(|| env.get("SERVER_AEAD_KEY").cloned())
        .expect("SERVER_AEAD_KEY must be set (base64url-no-pad)");
    let server_aead_key = URL_SAFE_NO_PAD
        .decode(server_aead_key_b64.trim())
        .expect("SERVER_AEAD_KEY base64url");

    let http = Client::new();

    // Deterministic test inputs
    let user_id = Uuid::new_v4();
    let password = format!("dev-pass-{}", Uuid::new_v4());

    // 1) OPAQUE registration (client side)
    let mut rng = rand::rngs::OsRng;
    let reg_start = ClientRegistration::<DefaultSuite>::start(&mut rng, password.as_bytes())
        .expect("opaque registration start");
    let reg_req_b64 = URL_SAFE_NO_PAD.encode(reg_start.message.serialize());

    let reg_init = http
        .post(format!("{base_url}/v1/auth/register/init"))
        .header("x-idempotency-key", format!("idem-{}", Uuid::new_v4()))
        .json(&RegisterInitRequest {
            user_id,
            registration_request: &reg_req_b64,
            credential_identifier: None,
        })
        .send()
        .await
        .expect("register/init http");
    let reg_init_body: SuccessEnvelope<RegisterInitResponse> =
        reg_init.json().await.expect("register/init json");

    let reg_resp_bytes = URL_SAFE_NO_PAD
        .decode(reg_init_body.data.registration_response.as_bytes())
        .expect("registration_response b64");
    let reg_resp = opaque_ke::RegistrationResponse::<DefaultSuite>::deserialize(&reg_resp_bytes)
        .expect("registration_response deserialize");
    let reg_upload = reg_start
        .state
        .finish(
            &mut rng,
            password.as_bytes(),
            reg_resp,
            ClientRegistrationFinishParameters::default(),
        )
        .expect("opaque registration finish");
    let reg_upload_b64 = URL_SAFE_NO_PAD.encode(reg_upload.message.serialize());

    // 2) Register finish (AEAD-wrapped)
    let mut signing_key_bytes = [0u8; 32];
    rng.fill_bytes(&mut signing_key_bytes);
    let signing_key = SigningKey::from_bytes(&signing_key_bytes);
    let ed_pub_b64 = URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes());
    let mut x25519_pub = [0u8; 32];
    rng.fill_bytes(&mut x25519_pub);
    let x25519_pub_b64 = URL_SAFE_NO_PAD.encode(x25519_pub);
    let mut kyber_pub = vec![0u8; 1184];
    rng.fill_bytes(&mut kyber_pub);
    let kyber_pub_b64 = URL_SAFE_NO_PAD.encode(kyber_pub);

    let mut emk_blob = vec![0u8; 64];
    rng.fill_bytes(&mut emk_blob);
    let mut enc_legal_name = vec![0u8; 32];
    rng.fill_bytes(&mut enc_legal_name);
    let mut enc_email = vec![0u8; 32];
    rng.fill_bytes(&mut enc_email);

    let reg_finish_inner = RegisterFinishRequest {
        session_id: reg_init_body.data.session_id,
        registration_upload: reg_upload_b64,
        ed25519_pubkey: ed_pub_b64,
        x25519_pubkey: x25519_pub_b64,
        kyber768_pubkey: kyber_pub_b64,
        emk_blob: URL_SAFE_NO_PAD.encode(emk_blob),
        argon2_params: serde_json::json!({"m": 65536, "t": 3, "p": 1}),
        enc_legal_name: URL_SAFE_NO_PAD.encode(enc_legal_name),
        enc_email: URL_SAFE_NO_PAD.encode(enc_email),
    };

    let reg_seq = 1u64;
    let reg_ts = now_ts();
    let reg_req_id = Uuid::new_v4().to_string();
    let reg_dev_id = Uuid::new_v4().to_string();
    let reg_finish_pt = serde_json::to_vec(&reg_finish_inner).expect("register/finish json");
    let reg_finish_env = encrypt_aead(
        &server_aead_key,
        &reg_req_id,
        reg_seq,
        reg_ts,
        &reg_finish_pt,
    );

    let reg_finish = http
        .put(format!("{base_url}/v1/auth/register/finish"))
        .header("x-idempotency-key", format!("idem-{}", Uuid::new_v4()))
        .header("x-request-id", reg_req_id.clone())
        .header("x-seq", reg_seq)
        .header("x-timestamp", reg_ts)
        .header("x-device-id", reg_dev_id)
        .json(&reg_finish_env)
        .send()
        .await
        .expect("register/finish http");

    let reg_finish_env: AeadEnvelope = reg_finish.json().await.expect("register/finish json");
    let reg_finish_envelope: SuccessEnvelope<RegisterFinishResponse> = decrypt_aead(
        &server_aead_key,
        &reg_req_id,
        reg_seq,
        reg_ts,
        &reg_finish_env,
    );
    assert_eq!(reg_finish_envelope.data.user_id, user_id);

    // 3) OPAQUE login
    let login_start = ClientLogin::<DefaultSuite>::start(&mut rng, password.as_bytes())
        .expect("opaque login start");
    let cred_req_b64 = URL_SAFE_NO_PAD.encode(login_start.message.serialize());

    let login_init = http
        .post(format!("{base_url}/v1/auth/login/init"))
        .header("x-idempotency-key", format!("idem-{}", Uuid::new_v4()))
        .json(&LoginInitRequest {
            user_id,
            credential_request: &cred_req_b64,
        })
        .send()
        .await
        .expect("login/init http");
    let login_init_body: SuccessEnvelope<LoginInitResponse> =
        login_init.json().await.expect("login/init json");

    let cred_resp_bytes = URL_SAFE_NO_PAD
        .decode(login_init_body.data.credential_response.as_bytes())
        .expect("credential_response b64");
    let cred_resp = opaque_ke::CredentialResponse::<DefaultSuite>::deserialize(&cred_resp_bytes)
        .expect("credential_response deserialize");

    let cred_fin = login_start
        .state
        .finish(
            &mut rng,
            password.as_bytes(),
            cred_resp,
            ClientLoginFinishParameters::default(),
        )
        .expect("opaque login finish");
    let cred_fin_b64 = URL_SAFE_NO_PAD.encode(cred_fin.message.serialize());

    let login_seq = 2u64;
    let login_ts = now_ts();
    let login_req_id = Uuid::new_v4().to_string();
    let login_dev_id = Uuid::new_v4().to_string();
    let login_finish_inner = LoginFinishRequest {
        session_id: login_init_body.data.session_id,
        credential_finalization: &cred_fin_b64,
    };
    let login_finish_pt = serde_json::to_vec(&login_finish_inner).expect("login/finish json");
    let login_finish_env = encrypt_aead(
        &server_aead_key,
        &login_req_id,
        login_seq,
        login_ts,
        &login_finish_pt,
    );

    let login_finish = http
        .post(format!("{base_url}/v1/auth/login/finish"))
        .header("x-idempotency-key", format!("idem-{}", Uuid::new_v4()))
        .header("x-request-id", login_req_id.clone())
        .header("x-seq", login_seq)
        .header("x-timestamp", login_ts)
        .header("x-device-id", login_dev_id)
        .json(&login_finish_env)
        .send()
        .await
        .expect("login/finish http");

    let login_finish_env: AeadEnvelope = login_finish.json().await.expect("login/finish json");
    let login_finish_envelope: SuccessEnvelope<LoginFinishResponse> = decrypt_aead(
        &server_aead_key,
        &login_req_id,
        login_seq,
        login_ts,
        &login_finish_env,
    );
    assert_eq!(login_finish_envelope.data.user_id, user_id);

    // 4) Device register + list (DB-backed)
    let device_id = Uuid::new_v4();
    let ts = now_ts();
    let canonical = transfer_legacy_crypto_core::jcs::canonicalize(&serde_json::json!({
        "device_id": device_id,
        "user_id": user_id,
        "ts": ts,
    }))
    .expect("canonicalize");
    let digest = transfer_legacy_crypto_core::hash::sha256(&canonical);
    let sig = signing_key.sign(&digest).to_bytes();

    let dev_inner = DeviceRegisterRequest {
        device_id,
        user_id,
        ts,
        device_sig: URL_SAFE_NO_PAD.encode(sig),
        ed25519_pubkey: URL_SAFE_NO_PAD.encode(signing_key.verifying_key().to_bytes()),
        device_meta: Some(serde_json::json!({"os":"e2e","arch":"test"})),
    };

    let dev_seq = 3u64;
    let dev_ts = now_ts();
    let dev_req_id = Uuid::new_v4().to_string();
    let dev_finish_pt = serde_json::to_vec(&dev_inner).expect("device/register json");
    let dev_env = encrypt_aead(
        &server_aead_key,
        &dev_req_id,
        dev_seq,
        dev_ts,
        &dev_finish_pt,
    );
    let _ = http
        .post(format!("{base_url}/v1/devices/register"))
        .header("x-idempotency-key", format!("idem-{}", Uuid::new_v4()))
        .header("x-request-id", dev_req_id)
        .header("x-seq", dev_seq)
        .header("x-timestamp", dev_ts)
        .header("x-device-id", device_id.to_string())
        .json(&dev_env)
        .send()
        .await
        .expect("devices/register http");

    let list = http
        .post(format!("{base_url}/v1/devices"))
        .json(&DeviceListRequest { user_id })
        .send()
        .await
        .expect("devices/list http");
    assert!(list.status().is_success());

    // If we got here, core DB-backed flows worked.
    println!("e2e_smoke_ok user_id={}", user_id);
}
