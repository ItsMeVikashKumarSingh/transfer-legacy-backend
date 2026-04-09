use zeroize::Zeroize;

#[derive(Clone, Debug)]
pub struct SensitiveBytes(Vec<u8>);

impl SensitiveBytes {
    pub fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    pub fn as_slice(&self) -> &[u8] {
        self.0.as_slice()
    }

    pub fn into_vec(mut self) -> Vec<u8> {
        let out = std::mem::take(&mut self.0);
        self.0.zeroize();
        out
    }
}

impl Drop for SensitiveBytes {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
