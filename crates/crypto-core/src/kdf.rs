use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Argon2Params {
    pub m: u32,
    pub t: u32,
    pub p: u32,
    pub salt: String,
}

impl Argon2Params {
    pub fn desktop_defaults(salt_b64: String) -> Self {
        Self {
            m: 262144, // 256MB
            t: 2,
            p: 2,
            salt: salt_b64,
        }
    }

    pub fn mobile_defaults(salt_b64: String) -> Self {
        Self {
            m: 65536, // 64MB
            t: 2,
            p: 1,
            salt: salt_b64,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self.m < 16384 {
            return Err("memory parameter 'm' too low");
        }
        if self.t < 1 {
            return Err("iterations parameter 't' must be >= 1");
        }
        if self.p < 1 {
            return Err("parallelism parameter 'p' must be >= 1");
        }
        if self.salt.is_empty() {
            return Err("salt cannot be empty");
        }
        Ok(())
    }
}
