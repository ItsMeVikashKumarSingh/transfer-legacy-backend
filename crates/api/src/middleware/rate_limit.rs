use axum::http::HeaderMap;
use redis::AsyncCommands;

use crate::errors::ApiError;
use crate::state::AppState;
use transfer_legacy_shared_types::AppError;

pub async fn require_idempotency(state: &AppState, headers: &HeaderMap) -> Result<(), ApiError> {
    let key = headers
        .get("x-idempotency-key")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| ApiError::app(AppError::BadRequest))?;

    let mut conn = state.redis_conn.clone();
    let redis_key = format!("idem:{}", key);

    let set_res: Result<bool, redis::RedisError> = conn.set_nx(&redis_key, 1).await;
    let set: bool = match set_res {
        Ok(val) => Ok(val),
        Err(e) => {
            tracing::warn!("require_idempotency: Redis set_nx failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            retry_conn.set_nx(&redis_key, 1).await.map_err(|err| {
                tracing::error!("require_idempotency: Redis set_nx retry failed for key '{}': {:?}", redis_key, err);
                ApiError::app(AppError::Internal)
            })
        }
    }?;

    if !set {
        return Err(ApiError::app(AppError::Conflict));
    }

    let exp_res: Result<(), redis::RedisError> = conn.expire(&redis_key, 86400).await;
    let _: () = match exp_res {
        Ok(_) => Ok(()),
        Err(e) => {
            tracing::warn!("require_idempotency: Redis expire failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            retry_conn.expire(&redis_key, 86400).await.map_err(|err| {
                tracing::error!("require_idempotency: Redis expire retry failed for key '{}': {:?}", redis_key, err);
                ApiError::app(AppError::Internal)
            })
        }
    }?;

    Ok(())
}

pub async fn enforce_rate_limit(
    state: &AppState,
    key: &str,
    max_per_minute: u64,
) -> Result<(), ApiError> {
    let mut conn = state.redis_conn.clone();
    let redis_key = format!("rate:{}", key);

    let incr_res: Result<u64, redis::RedisError> = conn.incr(&redis_key, 1).await;
    let count: u64 = match incr_res {
        Ok(val) => Ok(val),
        Err(e) => {
            tracing::warn!("enforce_rate_limit: Redis incr failed, retrying once. Error: {:?}", e);
            let mut retry_conn = state.redis_conn.clone();
            retry_conn.incr(&redis_key, 1).await.map_err(|err| {
                tracing::error!("enforce_rate_limit: Redis incr retry failed for key '{}': {:?}", redis_key, err);
                ApiError::app(AppError::Internal)
            })
        }
    }?;

    if count == 1 {
        let exp_res: Result<(), redis::RedisError> = conn.expire(&redis_key, 60).await;
        let _: () = match exp_res {
            Ok(_) => Ok(()),
            Err(e) => {
                tracing::warn!("enforce_rate_limit: Redis expire failed, retrying once. Error: {:?}", e);
                let mut retry_conn = state.redis_conn.clone();
                retry_conn.expire(&redis_key, 60).await.map_err(|err| {
                    tracing::error!("enforce_rate_limit: Redis expire retry failed for key '{}': {:?}", redis_key, err);
                    ApiError::app(AppError::Internal)
                })
            }
        }?;
    }

    if count > max_per_minute {
        return Err(ApiError::app(AppError::RateLimited));
    }
    Ok(())
}
