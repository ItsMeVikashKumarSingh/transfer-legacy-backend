use axum::extract::{Extension, State};
use axum::{http::HeaderMap, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{Rng, RngCore};
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::auth::{insert_opaque_record, insert_person_and_link, OpaqueRecordRow};
use crate::errors::{success, ApiError, SuccessEnvelope};
use crate::middleware::aead_transport::{wrap_response, AeadJson, AeadResponse};
use crate::middleware::rate_limit::{enforce_rate_limit, require_idempotency};
use crate::state::AppState;
use serde_json::Value;
use transfer_legacy_crypto_core::opaque::{registration_finish, registration_start};
use transfer_legacy_shared_types::{CURRENT_CRYPTO_VERSION, CURRENT_SCHEMA_VERSION};

#[derive(Debug, Deserialize)]
pub struct RegisterInitRequest {
    pub user_id: Uuid,
    pub registration_request: String,
    pub credential_identifier: Option<String>,
    pub verification_token: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterInitResponse {
    pub session_id: Uuid,
    pub registration_response: String,
    pub server_nonce: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterFinishRequest {
    pub session_id: Uuid,
    pub registration_upload: String,
    pub ed25519_pubkey: String,
    pub x25519_pubkey: String,
    pub kyber768_pubkey: String,
    pub emk_blob: String,
    pub argon2_params: Value,
    pub enc_legal_name: String,
    pub enc_email: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterFinishResponse {
    pub user_id: Uuid,
    pub person_id: Uuid,
}

#[derive(Debug, Serialize, Deserialize)]
struct RegisterSession {
    user_id: Uuid,
    credential_identifier: String,
}

pub async fn register_init(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<RegisterInitRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    
    require_idempotency(&state, &headers).await?;
    
    let rate_key = format!("register_init:{}", payload.user_id);
    enforce_rate_limit(&state, &rate_key, 10).await?;

    let credential_identifier = payload
        .credential_identifier
        .clone()
        .unwrap_or_else(|| payload.user_id.to_string());

    let config = state.config().await;

    // Email verification check (using Redis cached verification token)
    let bypass_otp = config.environment == crate::config::Environment::Local
        && payload.verification_token == "test-bypass-token";

    if !bypass_otp {
        let verify_key = format!("verified:email:{}", credential_identifier);
        let mut conn = state.redis_conn.clone();
        
        let cached_token: Option<String> = conn.get(&verify_key).await.map_err(|e| {
            tracing::error!("Failed to fetch verification token from Redis: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

        let cached_token = cached_token.ok_or_else(|| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid)
        })?;

        if cached_token != payload.verification_token {
            return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
        }

        // Delete token to prevent reuse
        let _: () = conn.del(&verify_key).await.map_err(|e| {
            tracing::warn!("Failed to delete registration verification token: {:?}", e);
        }).unwrap_or_default();
    }

    // Check if the user email already exists in auth.users
    let existing_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM auth.users WHERE email = $1")
        .bind(&credential_identifier)
        .fetch_optional(state.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to check existing email in auth.users: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    if let Some(old_uid) = existing_id {
        // Check if a completed OPAQUE record exists for this user ID
        let has_opaque: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_ext.opaque_records WHERE user_id = $1)")
            .bind(old_uid)
            .fetch_one(state.db())
            .await
            .map_err(|e| {
                tracing::error!("Failed to check existing opaque record: {:?}", e);
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            })?;

        if has_opaque {
            // Already registered!
            return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid));
        } else {
            // Incomplete registration, delete the old incomplete user in Supabase Auth Admin API
            crate::services::supabase::delete_user_in_supabase(&config, old_uid).await.map_err(|e| {
                tracing::error!("Failed to delete incomplete registration in Supabase: {:?}", e);
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            })?;
        }
    }

    // Now insert the new user record safely via Supabase Auth Admin API to satisfy foreign key constraints
    crate::services::supabase::register_user_in_supabase(&config, payload.user_id, &credential_identifier).await.map_err(|e| {
        match e {
            crate::services::supabase::SupabaseError::UserAlreadyExists => {
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid)
            }
            _ => {
                tracing::error!("Failed to create user in Supabase: {:?}", e);
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            }
        }
    })?;

    let (registration_response, _req) =
        registration_start(&state.opaque_setup, &payload.registration_request).map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let session_id = Uuid::new_v4();
    let mut nonce_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let server_nonce = URL_SAFE_NO_PAD.encode(nonce_bytes);

    let session = RegisterSession {
        user_id: payload.user_id,
        credential_identifier,
    };
    let mut conn = state.redis_conn.clone();

    let key = format!("opaque:reg:{}", session_id);
    let value = serde_json::to_string(&session).map_err(|e| {
        tracing::error!("register_init: Failed to serialize OPAQUE session: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    
    // SET_EX with 1 retry to handle idle connection drops gracefully
    let set_res: Result<(), redis::RedisError> = conn.set_ex(&key, value.clone(), 300).await;
    let _: () = match set_res {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!("register_init: Redis set_ex failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            let retry_res: Result<(), redis::RedisError> = retry_conn.set_ex(&key, value, 300).await;
            retry_res.map_err(|err| {
                tracing::error!("register_init: Redis set_ex retry failed for key '{}': {:?}", key, err);
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            })
        }
    }?;

    let envelope = SuccessEnvelope {
        data: RegisterInitResponse {
            session_id,
            registration_response,
            server_nonce,
        },
        request_id: rid,
    };
    let aead = wrap_response(&state.config().await, &headers, &envelope)?;
    Ok(Json(aead))
}

pub async fn register_finish(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    AeadJson(payload): AeadJson<RegisterFinishRequest>,
) -> Result<Json<AeadResponse>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    let config = state.config().await;

    require_idempotency(&state, &headers).await?;
    let mut conn = state.redis_conn.clone();
    let key = format!("opaque:reg:{}", payload.session_id);
    let session_json: Option<String> = conn.get(&key).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;
    let session_json = session_json.ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let session: RegisterSession = serde_json::from_str(&session_json).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let opaque_record = registration_finish(&state.opaque_setup, &payload.registration_upload)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;

    let ed25519_pubkey = URL_SAFE_NO_PAD
        .decode(payload.ed25519_pubkey)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let x25519_pubkey = URL_SAFE_NO_PAD.decode(payload.x25519_pubkey).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let kyber768_pubkey = URL_SAFE_NO_PAD
        .decode(payload.kyber768_pubkey)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let emk_blob = URL_SAFE_NO_PAD.decode(payload.emk_blob).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;
    let enc_legal_name = URL_SAFE_NO_PAD
        .decode(payload.enc_legal_name)
        .map_err(|_| {
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
        })?;
    let enc_email = URL_SAFE_NO_PAD.decode(payload.enc_email).map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::BadRequest, &rid)
    })?;

    let mut tx = state.db.begin().await.map_err(|e| {
        tracing::error!("Failed to begin transaction: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let person_id = insert_person_and_link(&mut tx, session.user_id, enc_legal_name, enc_email)
        .await
        .map_err(|e| {
            tracing::error!("Failed to insert person and link: {:?}", e);
            if let sqlx::Error::Database(ref db_err) = e {
                if db_err.code().as_deref() == Some("23505") {
                    return ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid);
                }
            }
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    let row = OpaqueRecordRow {
        user_id: session.user_id,
        opaque_record,
        emk_blob,
        argon2_params: payload.argon2_params,
        ed25519_pubkey,
        x25519_pubkey,
        kyber768_pubkey,
        crypto_version: CURRENT_CRYPTO_VERSION.as_str().to_string(),
        schema_version: CURRENT_SCHEMA_VERSION,
    };
    insert_opaque_record(&mut tx, &row).await.map_err(|e| {
        tracing::error!("Failed to insert opaque record: {:?}", e);
        if let sqlx::Error::Database(ref db_err) = e {
            if db_err.code().as_deref() == Some("23505") {
                return ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid);
            }
        }
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    tx.commit().await.map_err(|e| {
        tracing::error!("Failed to commit transaction: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let envelope = crate::errors::SuccessEnvelope {
        data: RegisterFinishResponse {
            user_id: session.user_id,
            person_id,
        },
        request_id: rid,
    };
    let aead = wrap_response(&config, &headers, &envelope)?;
    Ok(Json(aead))
}

#[derive(Debug, Deserialize)]
pub struct SendOtpRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct VerifyOtpRequest {
    pub email: String,
    pub code: String,
}

#[derive(Debug, Serialize)]
pub struct SendOtpResponse {
    pub status: &'static str,
}

#[derive(Debug, Serialize)]
pub struct VerifyOtpResponse {
    pub verification_token: String,
}

pub async fn register_send_otp(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<SendOtpRequest>,
) -> Result<Json<SuccessEnvelope<SendOtpResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await?;
    
    let rate_key = format!("otp_send:{}", payload.email);
    enforce_rate_limit(&state, &rate_key, 5).await?;

    // Check if the user email already exists in auth.users
    let existing_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM auth.users WHERE email = $1")
        .bind(&payload.email)
        .fetch_optional(state.db())
        .await
        .map_err(|e| {
            tracing::error!("Failed to check existing email in auth.users during OTP: {:?}", e);
            ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
        })?;

    if let Some(old_uid) = existing_id {
        // Check if a completed OPAQUE record exists for this user ID
        let has_opaque: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth_ext.opaque_records WHERE user_id = $1)")
            .bind(old_uid)
            .fetch_one(state.db())
            .await
            .map_err(|e| {
                tracing::error!("Failed to check existing opaque record during OTP: {:?}", e);
                ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
            })?;

        if has_opaque {
            // Already registered!
            return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Conflict, &rid));
        }
    }

    let otp_code = format!("{:06}", rand::thread_rng().gen_range(0..1000000));

    let redis_key = format!("otp:reg:{}", payload.email);
    let mut conn = state.redis_conn.clone();
    
    let _: () = conn.set_ex(&redis_key, &otp_code, 300).await.map_err(|e| {
        tracing::error!("Failed to store OTP in Redis: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let template = crate::notifications::resend::NotificationTemplate::RegisterOtp {
        code: otp_code,
    };
    
    state.notify(Uuid::nil(), &payload.email, template).await.map_err(|e| {
        tracing::error!("Failed to send OTP email: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(&rid, SendOtpResponse { status: "ok" }))
}

pub async fn register_verify_otp(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Json(payload): Json<VerifyOtpRequest>,
) -> Result<Json<SuccessEnvelope<VerifyOtpResponse>>, ApiError> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);
    require_idempotency(&state, &headers).await?;
    
    let rate_key = format!("otp_verify:{}", payload.email);
    enforce_rate_limit(&state, &rate_key, 5).await?;

    let redis_key = format!("otp:reg:{}", payload.email);
    let mut conn = state.redis_conn.clone();

    let cached_otp: Option<String> = conn.get(&redis_key).await.map_err(|e| {
        tracing::error!("Failed to fetch OTP from Redis: {:?}", e);
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let cached_otp = cached_otp.ok_or_else(|| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &rid)
    })?;

    if cached_otp != payload.code {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Unauthorized, &rid));
    }

    let _: () = conn.del(&redis_key).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    let mut token_bytes = [0u8; 32];
    rand::rngs::OsRng.fill_bytes(&mut token_bytes);
    let verification_token = URL_SAFE_NO_PAD.encode(token_bytes);

    let verify_key = format!("verified:email:{}", payload.email);
    let _: () = conn.set_ex(&verify_key, &verification_token, 600).await.map_err(|_| {
        ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &rid)
    })?;

    Ok(success(&rid, VerifyOtpResponse { verification_token }))
}

