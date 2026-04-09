use chacha20poly1305::{aead::{Aead, KeyInit}, XChaCha20Poly1305, XNonce};
use rand::RngCore;
use crate::memory::SensitiveBytes;

#[derive(thiserror::Error, Debug)]
pub enum AeadError {
    #[error("invalid key length")]
    InvalidKey,
    #[error("invalid nonce length")]
    InvalidNonce,
    #[error("encrypt error")]
    Encrypt,
    #[error("decrypt error")]
    Decrypt,
}

pub struct AeadEnvelope {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

pub fn encrypt(key: &[u8], plaintext: &[u8], aad: &[u8]) -> Result<AeadEnvelope, AeadError> {
    if key.len() != 32 {
        return Err(AeadError::InvalidKey);
    }
    let mut nonce = [0u8; 24];
    rand::rngs::OsRng.fill_bytes(&mut nonce);
    let cipher = XChaCha20Poly1305::new_from_slice(key).map_err(|_| AeadError::InvalidKey)?;
    let ciphertext = cipher
        .encrypt(XNonce::from_slice(&nonce), chacha20poly1305::aead::Payload { msg: plaintext, aad })
        .map_err(|_| AeadError::Encrypt)?;

    Ok(AeadEnvelope {
        nonce: nonce.to_vec(),
        ciphertext,
    })
}

pub fn decrypt(key: &[u8], nonce: &[u8], ciphertext: &[u8], aad: &[u8]) -> Result<Vec<u8>, AeadError> {
    if key.len() != 32 {
        return Err(AeadError::InvalidKey);
    }
    if nonce.len() != 24 {
        return Err(AeadError::InvalidNonce);
    }
    let key_protected = SensitiveBytes::new(key.to_vec());
    let cipher = XChaCha20Poly1305::new_from_slice(key_protected.as_slice()).map_err(|_| AeadError::InvalidKey)?;
    let plaintext = cipher
        .decrypt(XNonce::from_slice(nonce), chacha20poly1305::aead::Payload { msg: ciphertext, aad })
        .map_err(|_| AeadError::Decrypt)?;
    Ok(plaintext)
}
