use ed25519_dalek::{Signature, VerifyingKey, Verifier};

#[derive(thiserror::Error, Debug)]
pub enum SignatureError {
    #[error("invalid key")]
    InvalidKey,
    #[error("invalid signature")]
    InvalidSignature,
}

pub fn verify_ed25519(pubkey: &[u8], message: &[u8], signature: &[u8]) -> Result<(), SignatureError> {
    let key = VerifyingKey::from_bytes(pubkey.try_into().map_err(|_| SignatureError::InvalidKey)?)
        .map_err(|_| SignatureError::InvalidKey)?;
    let sig = Signature::from_bytes(signature.try_into().map_err(|_| SignatureError::InvalidSignature)?);
    key.verify(message, &sig).map_err(|_| SignatureError::InvalidSignature)
}
