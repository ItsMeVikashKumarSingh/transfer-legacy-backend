use redis::AsyncCommands;

use crate::state::AppState;

#[derive(thiserror::Error, Debug)]
pub enum FraudError {
    #[error("redis error")]
    Redis,
}

pub async fn bump_fraud_counter(state: &AppState, signal: &str) -> Result<u64, FraudError> {
    let mut conn = state
        .redis
        .get_multiplexed_async_connection()
        .await
        .map_err(|_| FraudError::Redis)?;
    let key = format!("fraud:{}", signal);
    let count: u64 = conn.incr(&key, 1).await.map_err(|_| FraudError::Redis)?;
    if count == 1 {
        let _: () = conn
            .expire(&key, 3600)
            .await
            .map_err(|_| FraudError::Redis)?;
    }
    Ok(count)
}
