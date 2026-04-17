use crate::errors::SuccessEnvelope;
use axum_test::TestServer;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use opaque_ke::{
    ClientLogin, ClientLoginFinishParameters, ClientLoginStartResult, ClientRegistration,
    ClientRegistrationFinishParameters, ClientRegistrationStartResult, CredentialResponse,
    RegistrationResponse,
};
use rand::rngs::OsRng;
use serde::{de::DeserializeOwned, Serialize};
use sqlx::PgPool;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::RwLock;
use transfer_legacy_crypto_core::opaque::{create_server_setup, server_setup_to_b64, DefaultSuite};
use uuid::Uuid;

use crate::config::Config;
use crate::state::AppState;
use axum::http::HeaderValue;
use transfer_legacy_crypto_core::aead::{decrypt, encrypt};

pub struct TestContext {
    pub client: Arc<TestServer>,
    pub db: PgPool,
    pub config: Config,
    pub state: AppState,
}

pub async fn spawn_app() -> TestContext {
    // Attempt to load from environment first (for local tests with .env.local),
    // then fallback to real OpenBao load if needed.
    let config = if let Ok(c) = Config::from_env() {
        c
    } else {
        Config::load()
            .await
            .expect("Failed to load config from both Env and OpenBao.")
    };

    let pool = PgPool::connect(&config.database_url)
        .await
        .expect("Failed to connect to test DB");

    // Automatically run migrations for fresh test environments, skip if requested
    if std::env::var("SKIP_MIGRATIONS").is_err() {
        sqlx::migrate!("../../migrations")
            .run(&pool)
            .await
            .expect("Failed to run migrations");
    }

    let state = AppState {
        config: Arc::new(RwLock::new(config.clone())),
        db: pool.clone(),
        redis: redis::Client::open(config.redis_url.as_str()).unwrap(),
        opaque_setup: transfer_legacy_crypto_core::opaque::server_setup_from_b64(
            &config.opaque_server_setup_b64,
        )
        .expect("Mock OPAQUE fail"),
    };

    let app = crate::router::create_router(&config, state.clone());
    let client = Arc::new(TestServer::new(app).expect("Failed to create TestServer"));

    TestContext {
        client,
        db: pool,
        config,
        state,
    }
}

pub struct CryptoClient {
    pub inner: Arc<TestServer>,
    pub config: Config,
    pub seq: AtomicU64,
    pub device_id: String,
    pub token: Option<String>,
}

impl CryptoClient {
    pub fn new(ctx: &TestContext) -> Self {
        Self {
            inner: ctx.client.clone(),
            config: ctx.config.clone(),
            seq: AtomicU64::new(1),
            device_id: Uuid::new_v4().to_string(),
            token: None,
        }
    }

    pub fn set_token(&mut self, token: String) {
        self.token = Some(token);
    }

    pub async fn post_aead<T, R>(&mut self, path: &str, body: &T) -> R
    where
        T: Serialize,
        R: DeserializeOwned,
    {
        let req_id = Uuid::new_v4().to_string();
        let idem_key = Uuid::new_v4().to_string();
        let ts = chrono::Utc::now().timestamp().to_string();
        let seq = self.seq.fetch_add(1, Ordering::SeqCst);
        let seq_str = seq.to_string();

        let aad = format!("{req_id}|{seq_str}|{ts}").into_bytes();
        let key = URL_SAFE_NO_PAD
            .decode(&self.config.server_aead_key_b64)
            .unwrap();
        let plaintext = serde_json::to_vec(body).unwrap();
        let enc = encrypt(&key, &plaintext, &aad).unwrap();

        let envelope = serde_json::json!({
            "nonce": URL_SAFE_NO_PAD.encode(enc.nonce),
            "ciphertext": URL_SAFE_NO_PAD.encode(enc.ciphertext),
        });

        let mut req = self.inner.post(path);

        req = req.add_header("x-request-id", &req_id);
        req = req.add_header("x-idempotency-key", &idem_key);
        req = req.add_header("x-seq", &seq_str);
        req = req.add_header("x-timestamp", &ts);
        req = req.add_header("x-device-id", &self.device_id);
        req = req.json(&envelope);

        if let Some(ref token) = self.token {
            req = req.add_header("Authorization", format!("Bearer {token}"));
        }

        let res = req.await;
        if !res.status_code().is_success() {
            panic!(
                "AEAD Request failed at {}: {} - {:?}",
                path,
                res.status_code(),
                res.text()
            );
        }

        let resp_env: crate::middleware::aead_transport::AeadResponse = res.json();
        let nonce = URL_SAFE_NO_PAD.decode(resp_env.nonce).unwrap();
        let ciphertext = URL_SAFE_NO_PAD.decode(resp_env.ciphertext).unwrap();

        let plain_resp =
            decrypt(&key, &nonce, &ciphertext, &aad).expect("Failed to decrypt response");

        let envelope: SuccessEnvelope<R> = serde_json::from_slice(&plain_resp).unwrap();
        envelope.data
    }
}

pub async fn create_test_user(db: &PgPool, user_id: Uuid, email: &str) {
    sqlx::query("INSERT INTO auth.users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING")
        .bind(user_id)
        .bind(email)
        .execute(db)
        .await
        .expect("Failed to create test user");
}

// Client-side OPAQUE helpers for testing
pub fn test_client_register_init(password: &str) -> (ClientRegistration<DefaultSuite>, String) {
    let mut rng = OsRng;
    let ClientRegistrationStartResult { message, state } =
        ClientRegistration::<DefaultSuite>::start(&mut rng, password.as_bytes()).unwrap();
    (state, URL_SAFE_NO_PAD.encode(message.serialize()))
}

pub fn test_client_register_finish(
    state: ClientRegistration<DefaultSuite>,
    response_b64: &str,
) -> String {
    let mut rng = OsRng;
    let resp_bytes = URL_SAFE_NO_PAD.decode(response_b64).unwrap();
    let response = RegistrationResponse::<DefaultSuite>::deserialize(&resp_bytes).unwrap();
    let result = state
        .finish(
            &mut rng,
            b"test-user",
            response,
            ClientRegistrationFinishParameters::default(),
        )
        .unwrap();
    URL_SAFE_NO_PAD.encode(result.message.serialize())
}

pub fn test_client_login_init(password: &str) -> (ClientLogin<DefaultSuite>, String) {
    let mut rng = OsRng;
    let ClientLoginStartResult { message, state } =
        ClientLogin::<DefaultSuite>::start(&mut rng, password.as_bytes()).unwrap();
    (state, URL_SAFE_NO_PAD.encode(message.serialize()))
}

pub fn test_client_login_finish(state: ClientLogin<DefaultSuite>, response_b64: &str) -> String {
    let mut rng = OsRng;
    let resp_bytes = URL_SAFE_NO_PAD.decode(response_b64).unwrap();
    let response = CredentialResponse::<DefaultSuite>::deserialize(&resp_bytes).unwrap();
    let result = state
        .finish(
            &mut rng,
            b"test-user",
            response,
            ClientLoginFinishParameters::default(),
        )
        .unwrap();
    URL_SAFE_NO_PAD.encode(result.message.serialize())
}
