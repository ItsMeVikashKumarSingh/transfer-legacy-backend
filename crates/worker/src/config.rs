use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub brevo_api_key: String,
    pub brevo_owner_reminder_early_template_id: String,
    pub brevo_owner_reminder_urgent_template_id: String,
    pub brevo_owner_reminder_daily_template_id: String,
    pub brevo_beneficiary_claim_template_id: String,
    pub brevo_approver_attestation_template_id: String,
    pub brevo_conflict_hold_template_id: String,
    pub brevo_release_ready_template_id: String,
    pub openbao_addr: String,
    pub openbao_token: String,
    pub b2_key_id: String,
    pub b2_app_key: String,
    pub b2_bucket_name: String,
    pub b2_audit_bucket_name: String,
    pub b2_backup_bucket_name: String,
    pub b2_endpoint_url: String,
    pub app_url: String,
    pub brand_name: String,
    pub server_id: String,
}

#[derive(thiserror::Error, Debug)]
pub enum ConfigError {
    #[error("missing required env var: {0}")]
    MissingVar(&'static str),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .map_err(|_| ConfigError::MissingVar("DATABASE_URL"))?,
            redis_url: env::var("REDIS_URL").map_err(|_| ConfigError::MissingVar("REDIS_URL"))?,
            brevo_api_key: env::var("BREVO_API_KEY")
                .map_err(|_| ConfigError::MissingVar("BREVO_API_KEY"))?,
            brevo_owner_reminder_early_template_id: env::var("BREVO_OWNER_REMINDER_EARLY_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_EARLY_TEMPLATE_ID"))?,
            brevo_owner_reminder_urgent_template_id: env::var("BREVO_OWNER_REMINDER_URGENT_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_URGENT_TEMPLATE_ID"))?,
            brevo_owner_reminder_daily_template_id: env::var("BREVO_OWNER_REMINDER_DAILY_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_OWNER_REMINDER_DAILY_TEMPLATE_ID"))?,
            brevo_beneficiary_claim_template_id: env::var("BREVO_BENEFICIARY_CLAIM_AVAILABLE_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_BENEFICIARY_CLAIM_AVAILABLE_TEMPLATE_ID"))?,
            brevo_approver_attestation_template_id: env::var("BREVO_APPROVER_ATTESTATION_REQUEST_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_APPROVER_ATTESTATION_REQUEST_TEMPLATE_ID"))?,
            brevo_conflict_hold_template_id: env::var("BREVO_CONFLICT_HOLD_NOTICE_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_CONFLICT_HOLD_NOTICE_TEMPLATE_ID"))?,
            brevo_release_ready_template_id: env::var("BREVO_RELEASE_READY_TEMPLATE_ID")
                .map_err(|_| ConfigError::MissingVar("BREVO_RELEASE_READY_TEMPLATE_ID"))?,
            openbao_addr: env::var("OPENBAO_ADDR")
                .map_err(|_| ConfigError::MissingVar("OPENBAO_ADDR"))?,
            openbao_token: env::var("OPENBAO_TOKEN")
                .map_err(|_| ConfigError::MissingVar("OPENBAO_TOKEN"))?,
            b2_key_id: env::var("BACKBLAZE_B2_KEY_ID")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_KEY_ID"))?,
            b2_app_key: env::var("BACKBLAZE_B2_APP_KEY")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_APP_KEY"))?,
            b2_bucket_name: env::var("BACKBLAZE_B2_BUCKET_NAME")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_BUCKET_NAME"))?,
            b2_audit_bucket_name: env::var("BACKBLAZE_B2_AUDIT_BUCKET_NAME")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_AUDIT_BUCKET_NAME"))?,
            b2_backup_bucket_name: env::var("BACKBLAZE_B2_BACKUP_BUCKET_NAME")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_BACKUP_BUCKET_NAME"))?,
            b2_endpoint_url: env::var("BACKBLAZE_B2_ENDPOINT_URL")
                .map_err(|_| ConfigError::MissingVar("BACKBLAZE_B2_ENDPOINT_URL"))?,
            app_url: env::var("TL_APP_URL").map_err(|_| ConfigError::MissingVar("TL_APP_URL"))?,
            brand_name: env::var("TL_BRAND_NAME")
                .map_err(|_| ConfigError::MissingVar("TL_BRAND_NAME"))?,
            server_id: env::var("TL_SERVER_ID").map_err(|_| ConfigError::MissingVar("TL_SERVER_ID"))?,
        })
    }
}
