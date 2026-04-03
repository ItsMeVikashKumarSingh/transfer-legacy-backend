use serde::Serialize;

#[derive(thiserror::Error, Debug)]
pub enum JcsError {
    #[error("canonicalization error")]
    Canonicalize,
}

pub fn canonicalize<T: Serialize>(value: &T) -> Result<Vec<u8>, JcsError> {
    serde_jcs::to_vec(value).map_err(|_| JcsError::Canonicalize)
}
