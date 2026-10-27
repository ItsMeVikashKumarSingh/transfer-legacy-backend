use axum::extract::{Extension, State};
use axum::Json;
use serde::Deserialize;
use tower_http::request_id::RequestId;

use crate::errors::{success, SuccessEnvelope};
use crate::services::ai::{AiPlannerResponse, AiPlannerService, ChatMessage, LegacyContextSummary};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct AiPlannerChatRequest {
    pub message: String,
    #[serde(default)]
    pub history: Vec<ChatMessage>,
    pub context: Option<LegacyContextSummary>,
}

pub async fn planner_chat_handler(
    State(state): State<AppState>,
    Extension(request_id): Extension<RequestId>,
    Json(payload): Json<AiPlannerChatRequest>,
) -> Json<SuccessEnvelope<AiPlannerResponse>> {
    let rid = crate::middleware::request_id::request_id_string(&request_id);

    let gemini_key = {
        let cfg = state.config.read().await;
        cfg.gemini_api_key.clone()
    };

    let service = AiPlannerService::new(gemini_key);
    let response = service
        .query(&payload.message, &payload.history, payload.context.as_ref())
        .await;

    success(&rid, response)
}
