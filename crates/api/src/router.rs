use axum::{routing::{get, post, put, delete}, Router};
use axum::http::{HeaderName, HeaderValue, Method};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::{CorsLayer, AllowOrigin};
use tower_http::limit::RequestBodyLimitLayer;
use tower_http::request_id::{MakeRequestUuid, RequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::state::AppState;
use crate::handlers::{health, capabilities};

pub fn create_router(config: &Config, state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(config.allowed_origins.clone()))
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            HeaderName::from_static("x-request-id"),
            HeaderName::from_static("x-seq"),
            HeaderName::from_static("x-timestamp"),
            HeaderName::from_static("x-device-id"),
            HeaderName::from_static("x-idempotency-key"),
        ]);

    let security_headers = ServiceBuilder::new()
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("strict-transport-security"),
            HeaderValue::from_static("max-age=63072000; includeSubDomains; preload"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-content-type-options"),
            HeaderValue::from_static("nosniff"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("x-frame-options"),
            HeaderValue::from_static("DENY"),
        ))
        .layer(SetResponseHeaderLayer::if_not_present(
            HeaderName::from_static("content-security-policy"),
            HeaderValue::from_static("default-src 'none'; frame-ancestors 'none';"),
        ));

    let middleware_stack = ServiceBuilder::new()
        .layer(TraceLayer::new_for_http())
        .layer(TimeoutLayer::new(Duration::from_secs(30)))
        .layer(RequestBodyLimitLayer::new(10 * 1024 * 1024))
        .layer(cors)
        .layer(security_headers)
        .layer(RequestIdLayer::new(HeaderName::from_static("x-request-id"), MakeRequestUuid))
        .layer(crate::middleware::sentry_layer::SentryLayer::new());

    let auth_routes = Router::new()
        .route("/register/init", post(crate::handlers::auth::register_init))
        .route("/register/finish", put(crate::handlers::auth::register_finish))
        .route("/login/init", post(crate::handlers::auth::login_init))
        .route("/login/finish", post(crate::handlers::auth::login_finish))
        .route("/logout", post(crate::handlers::auth::logout))
        .route("/refresh", post(crate::handlers::auth::refresh))
        .route("/password/reset/request", post(crate::handlers::auth::password_reset_request))
        .route("/password/reset/confirm", post(crate::handlers::auth::password_reset_confirm))
        .route("/mfa/totp/enroll", post(crate::handlers::auth::mfa::totp_enroll))
        .route("/mfa/totp/verify", post(crate::handlers::auth::mfa::totp_verify))
        .route("/mfa/webauthn/register/start", post(crate::handlers::auth::mfa::webauthn_register_start))
        .route("/mfa/webauthn/register/finish", post(crate::handlers::auth::mfa::webauthn_register_finish))
        .route("/mfa/webauthn/authenticate/start", post(crate::handlers::auth::mfa::webauthn_auth_start))
        .route("/mfa/webauthn/authenticate/finish", post(crate::handlers::auth::mfa::webauthn_auth_finish))
        .route("/stepup/request", post(crate::handlers::auth::stepup_request))
        .route("/stepup/verify", post(crate::handlers::auth::stepup_verify));

    let device_routes = Router::new()
        .route("/register", post(crate::handlers::devices::register))
        .route("/", post(crate::handlers::devices::list))
        .route("/:device_id", delete(crate::handlers::devices::revoke));

    let vault_routes = Router::new()
        .route("/items", post(crate::handlers::vault::create_item))
        .route("/items/list", post(crate::handlers::vault::list_items_handler))
        .route("/items/get", post(crate::handlers::vault::get_item_handler))
        .route("/items/delete", post(crate::handlers::vault::delete_item_handler))
        .route("/shares", post(crate::handlers::vault::create_share))
        .route("/shares/list", post(crate::handlers::vault::list_shares_handler))
        .route("/shares/revoke", post(crate::handlers::vault::revoke_share_handler))
        .route("/migrate", post(crate::handlers::vault::migrate_crypto));

    let inheritance_routes = Router::new()
        .route("/policy", put(crate::handlers::inheritance::upsert_policy))
        .route("/heartbeat", post(crate::handlers::inheritance::heartbeat))
        .route("/policy/:policy_id/invite", post(crate::handlers::inheritance::create_invite))
        .route("/claim-token/consume", post(crate::handlers::inheritance::consume_claim_token));

    Router::new()
        .route("/health", get(health))
        .route("/v1/server-capabilities", get(capabilities))
        .nest("/v1/auth", auth_routes)
        .nest("/v1/devices", device_routes)
        .nest("/v1/vault", vault_routes)
        .nest("/v1/inheritance", inheritance_routes)
        .with_state(state)
        .layer(middleware_stack)
}
