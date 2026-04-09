use axum::extract::{Extension, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::queries::claims::fetch_claim_for_update_tx;
use crate::db::queries::inheritance::fetch_policy_for_update_tx;
use crate::db::queries::vault::list_shares_for_grantee_owner;
use crate::errors::ApiError;
use crate::middleware::aead_transport::{AeadResponse, wrap_response};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct EnvelopesQuery {
    pub claim_id: Uuid,
    pub claimant_person_id: Uuid,
}

#[derive(Debug, Serialize)]
pub struct EnvelopeItem {
    pub share_id: Uuid,
    pub item_id: Uuid,
    pub envelope_b64: String,
    pub grant_sig_b64: String,
}

#[derive(Debug, Serialize)]
pub struct EnvelopesResponse {
    pub policy_id: Uuid,
    pub claim_id: Uuid,
    pub items: Vec<EnvelopeItem>,
}

pub async fn list_envelopes(
    State(state): State<AppState>,
    Extension(request_id): Extension<tower_http::request_id::RequestId>,
    headers: HeaderMap,
    Query(query): Query<EnvelopesQuery>,
) -> Result<Json<AeadResponse>, ApiError> {
    let mut tx = state.db.begin().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let claim = fetch_claim_for_update_tx(&mut tx, query.claim_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;
    if claim.claimant_person_id != query.claimant_person_id {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    let policy = fetch_policy_for_update_tx(&mut tx, claim.policy_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::NotFound, &request_id))?;
    if policy.status != "released" {
        return Err(ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Forbidden, &request_id));
    }

    tx.commit().await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let shares = list_shares_for_grantee_owner(&state.db, policy.owner_id, query.claimant_person_id)
        .await
        .map_err(|_| ApiError::app_with_request_id(transfer_legacy_shared_types::AppError::Internal, &request_id))?;

    let items = shares
        .into_iter()
        .map(|s| EnvelopeItem {
            share_id: s.share_id,
            item_id: s.item_id,
            envelope_b64: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.envelope),
            grant_sig_b64: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(s.grant_sig),
        })
        .collect();

    let envelope = crate::errors::SuccessEnvelope {
        data: EnvelopesResponse { policy_id: policy.policy_id, claim_id: query.claim_id, items },
        request_id: crate::middleware::request_id::request_id_string(&request_id),
    };
    let aead = wrap_response(&state, &headers, &envelope)?;
    Ok(Json(aead))
}
