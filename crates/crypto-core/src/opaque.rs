use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use opaque_ke::{
    ciphersuite::Default as DefaultSuite,
    CredentialRequest,
    CredentialResponse,
    RegistrationRequest,
    RegistrationResponse,
    RegistrationUpload,
    ServerLogin,
    ServerLoginStartResult,
    ServerRegistration,
    ServerSetup,
};
use rand::rngs::OsRng;

#[derive(thiserror::Error, Debug)]
pub enum OpaqueError {
    #[error("base64 decode error")]
    Base64,
    #[error("serialization error")]
    Serialization,
    #[error("opaque protocol error")]
    Protocol,
}

pub type OpaqueServerSetup = ServerSetup<DefaultSuite>;

pub fn create_server_setup() -> OpaqueServerSetup {
    let mut rng = OsRng;
    ServerSetup::new(&mut rng)
}

pub fn server_setup_to_b64(setup: &OpaqueServerSetup) -> String {
    let bytes = setup.serialize();
    URL_SAFE_NO_PAD.encode(bytes)
}

pub fn server_setup_from_b64(encoded: &str) -> Result<OpaqueServerSetup, OpaqueError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(encoded)
        .map_err(|_| OpaqueError::Base64)?;
    OpaqueServerSetup::deserialize(&bytes).map_err(|_| OpaqueError::Serialization)
}

pub fn registration_start(
    setup: &OpaqueServerSetup,
    registration_request_b64: &str,
) -> Result<(String, RegistrationRequest<DefaultSuite>), OpaqueError> {
    let req_bytes = URL_SAFE_NO_PAD
        .decode(registration_request_b64)
        .map_err(|_| OpaqueError::Base64)?;
    let req = RegistrationRequest::<DefaultSuite>::deserialize(&req_bytes)
        .map_err(|_| OpaqueError::Serialization)?;
    let mut rng = OsRng;
    let resp = ServerRegistration::start(&mut rng, req.clone(), setup)
        .map_err(|_| OpaqueError::Protocol)?;
    let resp_bytes = resp.message.serialize();
    Ok((URL_SAFE_NO_PAD.encode(resp_bytes), req))
}

pub fn registration_finish(
    setup: &OpaqueServerSetup,
    registration_upload_b64: &str,
) -> Result<Vec<u8>, OpaqueError> {
    let upload_bytes = URL_SAFE_NO_PAD
        .decode(registration_upload_b64)
        .map_err(|_| OpaqueError::Base64)?;
    let upload = RegistrationUpload::<DefaultSuite>::deserialize(&upload_bytes)
        .map_err(|_| OpaqueError::Serialization)?;
    let password_file = ServerRegistration::finish(upload, setup)
        .map_err(|_| OpaqueError::Protocol)?;
    Ok(password_file.serialize())
}

pub fn login_start(
    setup: &OpaqueServerSetup,
    credential_request_b64: &str,
    password_file: &[u8],
) -> Result<(String, ServerLogin<DefaultSuite>), OpaqueError> {
    let req_bytes = URL_SAFE_NO_PAD
        .decode(credential_request_b64)
        .map_err(|_| OpaqueError::Base64)?;
    let req = CredentialRequest::<DefaultSuite>::deserialize(&req_bytes)
        .map_err(|_| OpaqueError::Serialization)?;
    let password_file = opaque_ke::ServerRegistration::<DefaultSuite>::deserialize(password_file)
        .map_err(|_| OpaqueError::Serialization)?;
    let mut rng = OsRng;
    let ServerLoginStartResult { message, state } =
        ServerLogin::start(&mut rng, setup, password_file, req)
            .map_err(|_| OpaqueError::Protocol)?;
    let resp_bytes = message.serialize();
    Ok((URL_SAFE_NO_PAD.encode(resp_bytes), state))
}

pub fn login_finish(
    state: ServerLogin<DefaultSuite>,
    credential_finalization_b64: &str,
) -> Result<(), OpaqueError> {
    let fin_bytes = URL_SAFE_NO_PAD
        .decode(credential_finalization_b64)
        .map_err(|_| OpaqueError::Base64)?;
    let fin = opaque_ke::CredentialFinalization::<DefaultSuite>::deserialize(&fin_bytes)
        .map_err(|_| OpaqueError::Serialization)?;
    state
        .finish(fin)
        .map_err(|_| OpaqueError::Protocol)?;
    Ok(())
}

pub fn serialize_login_state(state: &ServerLogin<DefaultSuite>) -> Result<Vec<u8>, OpaqueError> {
    bincode::serialize(state).map_err(|_| OpaqueError::Serialization)
}

pub fn deserialize_login_state(bytes: &[u8]) -> Result<ServerLogin<DefaultSuite>, OpaqueError> {
    bincode::deserialize(bytes).map_err(|_| OpaqueError::Serialization)
}

pub fn serialize_registration_request(
    req: &RegistrationRequest<DefaultSuite>,
) -> Result<Vec<u8>, OpaqueError> {
    bincode::serialize(req).map_err(|_| OpaqueError::Serialization)
}

pub fn deserialize_registration_request(
    bytes: &[u8],
) -> Result<RegistrationRequest<DefaultSuite>, OpaqueError> {
    bincode::deserialize(bytes).map_err(|_| OpaqueError::Serialization)
}

pub fn serialize_credential_response(resp: &CredentialResponse<DefaultSuite>) -> Result<Vec<u8>, OpaqueError> {
    bincode::serialize(resp).map_err(|_| OpaqueError::Serialization)
}

pub fn serialize_registration_response(resp: &RegistrationResponse<DefaultSuite>) -> Result<Vec<u8>, OpaqueError> {
    bincode::serialize(resp).map_err(|_| OpaqueError::Serialization)
}
