use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridEnvelopeMeta {
    pub owner_id: String,
    pub item_id: String,
    pub crypto_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrantSignature {
    pub alg: String,
    pub sig: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HybridEnvelope {
    pub schema_version: u32,
    pub wrapper: String,
    pub owner_id: String,
    pub item_id: String,
    pub eph_x25519_pub: String,
    pub kyber_ct: String,
    pub nonce: String,
    pub payload: String,
    pub meta: HybridEnvelopeMeta,
    pub grant_sig: Option<GrantSignature>,
}

impl HybridEnvelope {
    pub fn validate_binding(
        &self,
        expected_owner: &str,
        expected_item: &str,
    ) -> Result<(), &'static str> {
        if self.wrapper != "hybrid-wrap-v1" {
            return Err("invalid wrapper type; expected hybrid-wrap-v1");
        }
        if self.owner_id != expected_owner || self.meta.owner_id != expected_owner {
            return Err("owner_id mismatch in envelope metadata");
        }
        if self.item_id != expected_item || self.meta.item_id != expected_item {
            return Err("item_id mismatch in envelope metadata");
        }
        Ok(())
    }
}
