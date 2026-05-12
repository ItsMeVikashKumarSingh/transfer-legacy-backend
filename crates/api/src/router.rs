use axum::http::{HeaderName, HeaderValue, Method};
use axum::middleware::{from_fn, from_fn_with_state};
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::set_header::SetResponseHeaderLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::config::Config;
use crate::handlers::{capabilities, health};
use crate::state::AppState;

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
        .layer(cors)
        .layer(security_headers)
        .layer(PropagateRequestIdLayer::new(HeaderName::from_static(
            "x-request-id",
        )))
        .layer(SetRequestIdLayer::new(
            HeaderName::from_static("x-request-id"),
            MakeRequestUuid,
        ))
        .layer(crate::middleware::sentry_layer::SentryLayer::new());

    let auth_routes = Router::new()
        .route("/register/init", post(crate::handlers::auth::register_init))
        .route(
            "/register/finish",
            post(crate::handlers::auth::register_finish),
        )
        .route("/login/init", post(crate::handlers::auth::login_init))
        .route("/login/finish", post(crate::handlers::auth::login_finish))
        .route("/logout", post(crate::handlers::auth::logout))
        .route("/refresh", post(crate::handlers::auth::refresh))
        .route(
            "/password/reset/request",
            post(crate::handlers::auth::password_reset_request),
        )
        .route(
            "/password/reset/confirm",
            post(crate::handlers::auth::password_reset_confirm),
        )
        .route(
            "/mfa/totp/enroll",
            post(crate::handlers::auth::mfa::totp_enroll),
        )
        .route(
            "/mfa/totp/verify",
            post(crate::handlers::auth::mfa::totp_verify),
        )
        .route(
            "/mfa/webauthn/register/start",
            post(crate::handlers::auth::mfa::webauthn_register_start),
        )
        .route(
            "/mfa/webauthn/register/finish",
            post(crate::handlers::auth::mfa::webauthn_register_finish),
        )
        .route(
            "/mfa/webauthn/authenticate/start",
            post(crate::handlers::auth::mfa::webauthn_auth_start),
        )
        .route(
            "/mfa/webauthn/authenticate/finish",
            post(crate::handlers::auth::mfa::webauthn_auth_finish),
        )
        .route(
            "/stepup/request",
            post(crate::handlers::auth::stepup_request),
        )
        .route("/stepup/verify", post(crate::handlers::auth::stepup_verify));

    let device_routes = Router::new()
        .route("/register", post(crate::handlers::devices::register))
        .route("/", post(crate::handlers::devices::list))
        .route("/:device_id", delete(crate::handlers::devices::revoke));

    let vault_routes = Router::new()
        .route("/items", post(crate::handlers::vault::create_item))
        .route(
            "/items/list",
            post(crate::handlers::vault::list_items_handler),
        )
        .route("/items/get", post(crate::handlers::vault::get_item_handler))
        .route(
            "/items/delete",
            post(crate::handlers::vault::delete_item_handler),
        )
        .route("/shares", post(crate::handlers::vault::create_share))
        .route(
            "/shares/list",
            post(crate::handlers::vault::list_shares_handler),
        )
        .route(
            "/shares/revoke",
            post(crate::handlers::vault::revoke_share_handler),
        )
        .route("/migrate", post(crate::handlers::vault::migrate_crypto));

    let inheritance_routes = Router::new()
        .route("/policy", put(crate::handlers::inheritance::upsert_policy))
        .route("/heartbeat", post(crate::handlers::inheritance::heartbeat))
        .route(
            "/policy/:policy_id/invite",
            post(crate::handlers::inheritance::create_invite),
        )
        .route(
            "/claim-token/consume",
            post(crate::handlers::inheritance::consume_claim_token),
        )
        .route(
            "/envelopes",
            get(crate::handlers::inheritance::list_envelopes),
        )
        .route(
            "/evidence-package",
            post(crate::handlers::inheritance::create_evidence_package),
        );

    let claims_routes = Router::new()
        .route("/initiate", post(crate::handlers::claims::initiate_claim))
        .route("/confirm", post(crate::handlers::claims::confirm_claim))
        .route(
            "/attachments/presign",
            post(crate::handlers::claims::presign_attachment),
        )
        .route(
            "/attachments/confirm",
            post(crate::handlers::claims::confirm_attachment),
        )
        .route(
            "/attestations",
            post(crate::handlers::claims::submit_attestation),
        )
        .route(
            "/release-records",
            post(crate::handlers::claims::create_release_record),
        );

    let audit_routes = Router::new().route("/chain", get(crate::handlers::audit::audit_chain));

    let gdpr_routes = Router::new()
        .route("/export", post(crate::handlers::gdpr::export_gdpr))
        .route("/erase", post(crate::handlers::gdpr::erase_gdpr));
        
    let app_routes = Router::new()
        .route("/branding", get(crate::handlers::app::get_branding))
        .route("/config", get(crate::handlers::app::get_branding))
        .route("/content/:slug", get(crate::handlers::app::get_content))
        .route("/waitlist", post(crate::handlers::app::waitlist_signup))
        .route("/pages", get(crate::handlers::app::list_pages))
        .route("/pages/:slug", get(crate::handlers::app::get_page))
        .route(
            "/branding",
            put(crate::handlers::app::update_branding_handler)
                .layer(from_fn_with_state(state.clone(), crate::middleware::internal_auth::administrative_auth)),
        )
        .route(
            "/content",
            put(crate::handlers::app::update_content_handler)
                .layer(from_fn_with_state(state.clone(), crate::middleware::internal_auth::administrative_auth)),
        )
        .route(
            "/waitlist",
            get(crate::handlers::app::list_waitlist_entries_handler)
                .layer(from_fn_with_state(state.clone(), crate::middleware::internal_auth::administrative_auth)),
        );

    let protected_ops_routes = Router::new()
        .route("/change-password", post(crate::handlers::ops::change_password_handler))
        .route("/admins", get(crate::handlers::ops::list_admins_handler))
        .route("/admins", post(crate::handlers::ops::create_admin_handler))
        .route("/admins/:id", delete(crate::handlers::ops::delete_admin_handler))
        .route("/roles", get(crate::handlers::ops::list_roles_handler))
        .route("/roles", post(crate::handlers::ops::create_role_handler))
        .route("/roles/:id", put(crate::handlers::ops::update_role_handler))
        .route("/roles/:id", delete(crate::handlers::ops::delete_role_handler))
        .route("/waitlist", get(crate::handlers::ops::list_waitlist_handler))
        .route("/branding", get(crate::handlers::ops::get_branding_handler))
        .route("/branding", put(crate::handlers::ops::update_branding_handler))
        .route("/contact", get(crate::handlers::ops::get_contact_handler))
        .route("/contact", put(crate::handlers::ops::update_contact_handler))
        .route("/contact/messages", get(crate::handlers::ops::list_contact_messages_handler))
        .route("/contact/messages/:id", delete(crate::handlers::ops::delete_contact_message_handler))
        .route("/content", put(crate::handlers::ops::update_content_ops_handler))
        .route("/pages", get(crate::handlers::ops::list_pages_ops))
        .route("/pages/:slug", get(crate::handlers::ops::get_page_ops))
        .route("/pages/:slug", put(crate::handlers::ops::upsert_page_ops))
        .route("/pages/:slug", delete(crate::handlers::ops::delete_page_ops))
        .route("/reviews", get(crate::handlers::ops::list_reviews))
        .route("/reviews/:review_id", get(crate::handlers::ops::get_review))
        .route(
            "/reviews/:review_id/decision",
            post(crate::handlers::ops::review_decision),
        )
        .route(
            "/storage/presigned-logo",
            post(crate::handlers::ops::get_presigned_logo_upload),
        )
        .route("/logs", get(crate::handlers::ops::list_audit_logs_handler))
        .layer(from_fn_with_state(
            state.clone(),
            crate::middleware::internal_auth::administrative_auth,
        ));

    let ops_routes = Router::new()
        .route("/login", post(crate::handlers::ops::login_handler))
        .nest("/", protected_ops_routes);

    let jobs_routes = Router::new()
        .route("/heartbeat-eval", post(crate::handlers::jobs::heartbeat_eval))
        .route("/audit-anchor", post(crate::handlers::jobs::audit_anchor))
        .route("/release-eval", post(crate::handlers::jobs::release_eval))
        .route("/conflict-check", post(crate::handlers::jobs::conflict_check))
        .route("/release-delivery", post(crate::handlers::jobs::release_delivery));

    Router::new()
        .route("/health", get(health))
        .route("/metrics", get(crate::handlers::metrics::metrics))
        .route("/v1/server-capabilities", get(capabilities))
        .route(
            "/v1/openapi.json",
            get(crate::handlers::openapi::openapi_json),
        )
        .route("/v1/docs", get(crate::handlers::openapi::docs_ui))
        .nest("/v1/auth", auth_routes)
        .nest("/v1/devices", device_routes)
        .nest("/v1/vault", vault_routes)
        .nest("/v1/inheritance", inheritance_routes)
        .nest("/v1/claims", claims_routes)
        .nest("/v1/audit", audit_routes)
        .nest("/v1/gdpr", gdpr_routes)
        .nest("/v1/app", app_routes)
        .nest("/v1/ops", ops_routes)
        .nest("/v1/jobs", jobs_routes)
        .with_state(state)
        .layer(from_fn(crate::middleware::metrics::metrics_middleware))
        .layer(middleware_stack)
}
