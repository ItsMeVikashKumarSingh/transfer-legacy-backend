use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(thiserror::Error, Debug)]
pub enum HmacError {
    #[error("hmac error")]
    Hmac,
}

pub fn compute_hmac(secret: &[u8], data: &[u8]) -> Result<Vec<u8>, HmacError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| HmacError::Hmac)?;
    mac.update(data);
    Ok(mac.finalize().into_bytes().to_vec())
}

pub fn verify_hmac(secret: &[u8], data: &[u8], expected: &[u8]) -> Result<(), HmacError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| HmacError::Hmac)?;
    mac.update(data);
    mac.verify_slice(expected).map_err(|_| HmacError::Hmac)
}
