use aws_config::Region;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum B2Error {
    #[error("upload error")]
    Upload,
}

pub async fn upload_anchor(config: &Config, key: &str, payload: Vec<u8>) -> Result<(), B2Error> {
    let creds = Credentials::new(
        config.b2_key_id.clone(),
        config.b2_app_key.clone(),
        None,
        None,
        "b2",
    );

    let conf = aws_sdk_s3::config::Builder::new()
        .region(Region::new("us-east-1"))
        .endpoint_url(config.b2_endpoint_url.clone())
        .credentials_provider(creds)
        .force_path_style(true)
        .build();

    let client = Client::from_conf(conf);
    client
        .put_object()
        .bucket(config.b2_audit_bucket_name.clone())
        .key(key)
        .body(ByteStream::from(payload))
        .send()
        .await
        .map_err(|_| B2Error::Upload)?;

    Ok(())
}
