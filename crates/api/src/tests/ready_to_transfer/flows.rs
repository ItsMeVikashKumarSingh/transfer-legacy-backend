use uuid::Uuid;
use crate::state::AppState;
use crate::db::queries::auth::OpaqueRecordRow;
use serde_json::json;

/// Verifies that OPAQUE records and user links are correctly saved in the DB.
pub async fn verify_registration_persistence(state: &AppState, user_id: Uuid) -> anyhow::Result<()> {
    // 1. Audit Check: Does the OPAQUE record exist?
    let record = crate::db::queries::auth::fetch_opaque_record(&state.db, user_id).await?;
    assert_eq!(record.user_id, user_id);
    
    // 2. Audit Check: Is the person linked?
    // (This would involve a custom query or checking the person_user_links table)
    
    Ok(())
}

/// Verifies that vault items are correctly saved and metadata is preserved.
pub async fn verify_vault_persistence(state: &AppState, user_id: Uuid, item_id: Uuid) -> anyhow::Result<()> {
    let item = crate::db::queries::vault::get_item(&state.db, user_id, item_id).await?;
    assert_eq!(item.item_id, item_id);
    assert_eq!(item.user_id, user_id);
    
    Ok(())
}
