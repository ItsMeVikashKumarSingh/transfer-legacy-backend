pub mod flows;

#[cfg(test)]
mod tests {
    use crate::config::{Config, Environment};
    use crate::db::queries::vault;
    use crate::notifications::resend::{send_notification, NotificationTemplate};
    use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
    use opaque_ke::{ClientRegistration, RegistrationResponse};
    use rand::rngs::OsRng;
    use sqlx::PgPool;
    use transfer_legacy_crypto_core::opaque::{
        create_server_setup, server_setup_from_b64, server_setup_to_b64, DefaultSuite,
    };
    use uuid::Uuid;

    #[tokio::test]
    async fn verify_full_lifecycle_success() {
        // 1. Initial Setup: Load real environment
        dotenvy::from_filename(".env.local").ok();

        // Ensure mandatory config vars have at least dummy values for the e2e test if missing
        if std::env::var("OPENBAO_ADDR").is_err() {
            std::env::set_var("OPENBAO_ADDR", "http://127.0.0.1:8200");
        }
        if std::env::var("ROLE_ID").is_err() {
            std::env::set_var("ROLE_ID", "test-role-id");
        }
        if std::env::var("SECRET_ID").is_err() {
            std::env::set_var("SECRET_ID", "test-secret-id");
        }

        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&db_url)
            .await
            .expect("Failed to connect to test Database");

        // Ensure database is ready for specialized end-to-end tests
        sqlx::migrate!("../../migrations").run(&pool).await.ok();

        let setup = if let Ok(setup_b64) = std::env::var("OPAQUE_SERVER_SETUP") {
            server_setup_from_b64(&setup_b64).expect("Failed to parse setup from env")
        } else {
            create_server_setup()
        };

        // 2. Client Side: Start Registration
        let mut rng = OsRng;
        let pswd = b"functional-test-password-2026";
        let client_reg_start = ClientRegistration::<DefaultSuite>::start(&mut rng, pswd)
            .expect("Failed to start client registration");
        let registration_request_b64 = URL_SAFE_NO_PAD.encode(client_reg_start.message.serialize());

        let test_user_id = Uuid::new_v4();
        println!("🚀 Starting Full Lifecycle Test for user: {}", test_user_id);

        // 3. Server Side: Register Init (Mocked handler logic for speed, but using real DB/Redis)
        // We simulate the register_init logic here
        let (registration_response_b64, _) =
            transfer_legacy_crypto_core::opaque::registration_start(
                &setup,
                &registration_request_b64,
            )
            .expect("Server failed registration_start");

        // 4. Client Side: Finish Registration
        let registration_response_bytes =
            URL_SAFE_NO_PAD.decode(&registration_response_b64).unwrap();
        let registration_response =
            RegistrationResponse::<DefaultSuite>::deserialize(&registration_response_bytes)
                .unwrap();

        let client_reg_finish = client_reg_start
            .state
            .finish(
                &mut rng,
                pswd,
                registration_response,
                Default::default(), // Using Default::default() usually resolves to () if applicable
            )
            .expect("Client failed registration_finish");

        let registration_upload_b64 = URL_SAFE_NO_PAD.encode(client_reg_finish.message.serialize());

        // 5. Server Side: Register Finish logic
        // We manually insert into auth_ext and core to verify the schemas
        let mut tx = pool.begin().await.unwrap();

        // Ensure user exists in auth.users
        sqlx::query("INSERT INTO auth.users (id, email) VALUES ($1, $2) ON CONFLICT DO NOTHING")
            .bind(test_user_id)
            .bind(format!("func_test_{}@example.com", test_user_id))
            .execute(&mut *tx)
            .await
            .expect("Failed to provision auth.user");

        let enc_name = URL_SAFE_NO_PAD.encode(b"Functional Test User");
        let enc_email = URL_SAFE_NO_PAD.encode(b"func_test@example.com");

        // Insert person
        let person_id: (Uuid,) = sqlx::query_as("INSERT INTO auth_ext.persons (enc_legal_name, enc_email) VALUES ($1, $2) RETURNING person_id")
            .bind(URL_SAFE_NO_PAD.decode(enc_name).unwrap())
            .bind(URL_SAFE_NO_PAD.decode(enc_email).unwrap())
            .fetch_one(&mut *tx)
            .await
            .expect("Failed to insert person");

        sqlx::query("INSERT INTO auth_ext.person_user_links (person_id, user_id) VALUES ($1, $2)")
            .bind(person_id.0)
            .bind(test_user_id)
            .execute(&mut *tx)
            .await
            .expect("Failed to link person");

        // Insert OPAQUE record
        let pswd_file = transfer_legacy_crypto_core::opaque::registration_finish(
            &setup,
            &registration_upload_b64,
        )
        .unwrap();
        sqlx::query("INSERT INTO auth_ext.opaque_records (user_id, opaque_record, emk_blob, argon2_params, crypto_version, schema_version, ed25519_pubkey, x25519_pubkey, kyber768_pubkey) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(test_user_id)
            .bind(pswd_file)
            .bind(vec![0u8; 32]) // Mock emk for test
            .bind(serde_json::json!({}))
            .bind("v1")
            .bind(1)
            .bind(vec![0u8; 32]) // Mock ed25519
            .bind(vec![0u8; 32]) // Mock x25519
            .bind(vec![0u8; 1184]) // Mock kyber768
            .execute(&mut *tx)
            .await
            .expect("Failed to insert opaque record");

        tx.commit().await.unwrap();
        println!("✅ OPAQUE Registration Flow Completed");

        // 6. Vault Persistence Test
        let test_item_id = Uuid::new_v4();
        let ciphertext = b"ultra-secure-functional-test-data".to_vec();

        sqlx::query(
            "INSERT INTO core.items (item_id, user_id, ciphertext, crypto_version, schema_version) VALUES ($1,$2,$3,$4,$5)"
        )
        .bind(test_item_id)
        .bind(test_user_id)
        .bind(&ciphertext)
        .bind("v1")
        .bind(1)
        .execute(&pool)
        .await
        .expect("Failed to persist vault item");

        println!("✅ Vault Item {} Persisted in 'core' schema", test_item_id);

        // 7. Verify Read
        let saved = vault::get_item(&pool, test_user_id, test_item_id)
            .await
            .expect("Failed to read back item");
        assert_eq!(saved.item_id, test_item_id);
        assert_eq!(saved.ciphertext, ciphertext);
        println!("✅ Success: Data read back verified");

        // 8. Real Resend Notification Verification
        println!("✉️ Testing REAL Resend Notification delivery...");

        let mut origins = Vec::new();
        origins.push(axum::http::HeaderValue::from_static(
            "http://localhost:3000",
        ));

        let config = Config {
            environment: Environment::Local,
            bind_addr: "127.0.0.1".to_string(),
            port: 8080,
            allowed_origins: origins,
            app_url: "http://localhost:8080".to_string(),
            brand_name: "Transfer Legacy".to_string(),
            internal_api_token: Some("test-token".to_string()),
            database_url: db_url,
            redis_url: "redis://127.0.0.1:6379/1".to_string(),
            bao_path: "secret/data/transfer-legacy/prod".to_string(),
            openbao_addr: "http://127.0.0.1:8200".to_string(),
            openbao_token: "test".to_string(),
            openbao_version: 1,
            b2_key_id: "test".to_string(),
            b2_app_key: "test".to_string(),
            b2_bucket_name: "test".to_string(),
            b2_public_assets_bucket_name: "test-assets".to_string(),
            b2_audit_bucket_name: "test-audit".to_string(),
            b2_backup_bucket_name: "test-backup".to_string(),
            b2_endpoint_url: "http://localhost".to_string(),
            server_aead_key_b64: URL_SAFE_NO_PAD.encode(vec![0u8; 32]),
            opaque_server_setup_b64: server_setup_to_b64(&setup),
            jwt_secret: "test-secret".to_string(),
            supabase_url: "http://localhost:54321".to_string(),
            supabase_publishable_key: "test".to_string(),
            supabase_secret_key: "test".to_string(),
            server_hmac_secret: "test-hmac".to_string(),
            resend_api_key: std::env::var("RESEND_API_KEY")
                .unwrap_or_else(|_| "test-key".to_string()),
            owner_email: "vikashbro111@gmail.com".to_string(),
            ops_admin_email: "admin@transferlegacy.com".to_string(),
            ops_admin_password: "Admin@123".to_string(),
            tl_serverless: false,
            server_private_key_b64: None,
            tl_cron_secret: None,
        };

        let test_email = "vikashbro111@gmail.com";
        let res = send_notification(
            &config,
            test_email,
            NotificationTemplate::Invite {
                owner_name: "Functional Test Owner".to_string(),
                policy_name: "E2E Test Policy".to_string(),
                invite_url: "http://localhost:8080/invite/claim?id=test".to_string(),
                invite_id: "func-test-invite".to_string(),
                claim_token: "test-token-123".to_string(),
                expires_at: "2026-12-31".to_string(),
            },
        )
        .await;

        match res {
            Ok(_) => println!("✅ REAL E-MAIL SENT SUCCESSFULLY to {}", test_email),
            Err(e) => panic!("❌ Resend Notification Failed: {}", e),
        }

        println!(
            "⭐ ALL SYSTEMS FUNCTIONAL: OPAQUE Handshake, core persistence, and Resend verified."
        );
    }

    #[tokio::test]
    async fn test_redis_connection() {
        let client = redis::Client::open("redis://127.0.0.1:6379").unwrap();
        match client.get_multiplexed_async_connection().await {
            Ok(_) => println!("RECONNECT SUCCESSFUL"),
            Err(e) => panic!("RECONNECT FAILED: {:?}", e),
        }
    }

    #[tokio::test]
    async fn test_delete_item_db() {
        dotenvy::from_filename(".env.local").ok();
        let db_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let pool = PgPool::connect(&db_url).await.unwrap();
        let user_id = Uuid::new_v4();
        let item_id = Uuid::new_v4();
        let ciphertext = b"test".to_vec();
        sqlx::query("INSERT INTO auth.users (id, email) VALUES ($1, $2)")
            .bind(user_id)
            .bind(format!("test_del_{}@example.com", user_id))
            .execute(&pool)
            .await
            .unwrap();

        sqlx::query(
            "INSERT INTO core.items (item_id, user_id, ciphertext, crypto_version, schema_version) VALUES ($1,$2,$3,$4,$5)"
        )
        .bind(item_id)
        .bind(user_id)
        .bind(&ciphertext)
        .bind("v1")
        .bind(1)
        .execute(&pool)
        .await
        .unwrap();

        match sqlx::query(
            "UPDATE core.items SET is_deleted = true, deleted_at = now() WHERE user_id = $1 AND item_id = $2",
        )
        .bind(user_id)
        .bind(item_id)
        .execute(&pool)
        .await {
            Ok(_) => println!("DELETE ITEM SQL SUCCESSFUL"),
            Err(e) => panic!("DELETE ITEM SQL FAILED: {:?}", e),
        }
    }
}
