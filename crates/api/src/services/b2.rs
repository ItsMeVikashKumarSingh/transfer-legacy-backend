use aws_config::Region;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::Client;
use std::time::Duration;

use crate::config::Config;

#[derive(thiserror::Error, Debug)]
pub enum B2Error {
    #[error("presign error")]
    Presign,
}

pub async fn presign_put(
    config: &Config,
    key: &str,
    content_type: &str,
    expires_secs: u64,
) -> Result<String, B2Error> {
    presign_put_to_bucket(config, &config.b2_bucket_name, key, content_type, expires_secs).await
}

pub async fn presign_put_to_bucket(
    config: &Config,
    bucket: &str,
    key: &str,
    content_type: &str,
    expires_secs: u64,
) -> Result<String, B2Error> {
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
    let presigned = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .content_type(content_type)
        .presigned(
            PresigningConfig::expires_in(Duration::from_secs(expires_secs))
                .map_err(|_| B2Error::Presign)?,
        )
        .await
        .map_err(|_| B2Error::Presign)?;

    Ok(presigned.uri().to_string())
}
