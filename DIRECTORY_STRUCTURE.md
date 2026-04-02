# Transfer Legacy — Final Directory Structure

> **Stack:** Rust · Axum · Supabase PostgreSQL · Redis · OpenBao · Backblaze B2 · Brevo · Hetzner CX22
> Generated from confirmed decisions in `project_detail.md` and `DEVELOPMENT_PLAN.md`

```
transfer-legacy/
│
├── Cargo.toml                              ← workspace root; lists all crate members
├── Cargo.lock
├── rust-toolchain.toml                     ← pin stable Rust version (e.g. 1.77+)
├── rustfmt.toml
├── .clippy.toml                            ← warn-on-all-lints = true
├── deny.toml                               ← cargo-deny: block GPL/AGPL + RUSTSEC advisories
├── .env.example                            ← template only — no real values ever
├── .gitignore
├── README.md
├── DEVELOPMENT_PLAN.md                     ← phased execution roadmap
│
├── crates/
│   │
│   ├── shared-types/                       ← Phase 0 · used by all crates
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── errors.rs                   ← AppError enum → RFC 7807 error codes
│   │       ├── crypto_types.rs             ← CryptoVersion enum, encoding helpers
│   │       ├── schema_versions.rs          ← CURRENT_SCHEMA_VERSION constants
│   │       └── models/
│   │           ├── mod.rs
│   │           ├── user.rs
│   │           ├── person.rs               ← person_id vs user_id (§2.4 project_detail.md)
│   │           ├── device.rs
│   │           ├── item.rs
│   │           ├── share.rs                ← ShareEnvelope struct
│   │           ├── policy.rs               ← PolicyStatus enum (all 8 states)
│   │           ├── claim.rs
│   │           ├── attestation.rs
│   │           └── release_record.rs       ← canonical JSON schema
│   │
│   ├── crypto-core/                        ← Phase 1–2 · native + WASM compatible
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── aead.rs                     ← XChaCha20-Poly1305 encrypt/decrypt
│   │       ├── kdf.rs                      ← Argon2id KEK derivation
│   │       ├── opaque.rs                   ← OPAQUE-ke server-side state (register/login)
│   │       ├── hybrid_kem.rs               ← X25519 + Kyber-768 envelope wrap/unwrap
│   │       ├── signatures.rs               ← Ed25519 sign/verify + optional Dilithium-2
│   │       ├── jcs.rs                      ← JCS RFC 8785 canonicalization
│   │       ├── memory.rs                   ← protected memory wrappers, zeroize on drop
│   │       ├── versions.rs                 ← version gate, reject deprecated versions
│   │       └── tests/
│   │           ├── aead_test.rs
│   │           ├── hybrid_kem_test.rs
│   │           ├── opaque_test.rs
│   │           ├── jcs_test.rs
│   │           ├── signatures_test.rs
│   │           └── fuzz/                   ← Phase 9 · cargo-fuzz targets
│   │               ├── fuzz_aead_decrypt.rs
│   │               ├── fuzz_envelope_parsing.rs
│   │               └── fuzz_jcs_canonicalization.rs
│   │
│   ├── api/                                ← Phase 0.9 onwards · Axum HTTP server
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── lib.rs
│   │       ├── config.rs                   ← loads all config from OpenBao KV at startup
│   │       ├── router.rs                   ← all route mounts + middleware stack
│   │       ├── telemetry.rs                ← Sentry + PostHog + metrics init
│   │       ├── errors.rs                   ← AppError → RFC 7807 HTTP response mapping
│   │       │
│   │       ├── middleware/
│   │       │   ├── mod.rs
│   │       │   ├── aead_transport.rs       ← XChaCha20 request/response wrap
│   │       │   ├── request_id.rs           ← inject X-Request-ID UUID
│   │       │   ├── security_headers.rs     ← HSTS, CSP, X-Frame-Options
│   │       │   ├── rate_limit.rs           ← Redis token bucket per IP + user
│   │       │   └── sentry_layer.rs         ← propagate request context to Sentry
│   │       │
│   │       ├── handlers/
│   │       │   ├── mod.rs
│   │       │   ├── health.rs               ← GET /health
│   │       │   ├── capabilities.rs         ← GET /v1/server-capabilities
│   │       │   │
│   │       │   ├── auth/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── register.rs         ← OPAQUE register init/finish
│   │       │   │   ├── login.rs            ← OPAQUE login init/finish
│   │       │   │   ├── logout.rs
│   │       │   │   ├── refresh.rs
│   │       │   │   ├── password.rs         ← reset request + confirm
│   │       │   │   └── mfa/
│   │       │   │       ├── mod.rs
│   │       │   │       ├── totp.rs         ← TOTP enroll/verify
│   │       │   │       └── webauthn.rs     ← WebAuthn register/authenticate
│   │       │   │
│   │       │   ├── devices/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── register.rs         ← device Ed25519 key registration
│   │       │   │   ├── list.rs
│   │       │   │   └── revoke.rs           ← step-up required
│   │       │   │
│   │       │   ├── vault/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── items.rs            ← CRUD: ciphertext only, no decrypt path
│   │       │   │   ├── shares.rs           ← pre-wrap envelope store/list/revoke
│   │       │   │   └── migrate.rs          ← crypto version migration endpoint
│   │       │   │
│   │       │   ├── inheritance/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── policy.rs           ← create/update policy
│   │       │   │   ├── heartbeat.rs        ← signed heartbeat submission
│   │       │   │   ├── invite.rs           ← invite beneficiary/approver
│   │       │   │   ├── claim.rs            ← initiate claim
│   │       │   │   ├── attest.rs           ← approver attestation submission
│   │       │   │   └── envelopes.rs        ← post-release envelope fetch
│   │       │   │
│   │       │   ├── files/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── presign.rs          ← Backblaze B2 presigned PUT URL
│   │       │   │   └── confirm.rs          ← hash verify + attach to claim
│   │       │   │
│   │       │   ├── audit/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── chain.rs            ← GET audit chain (paginated)
│   │       │   │   ├── verify.rs           ← chain integrity check
│   │       │   │   └── evidence.rs         ← evidence package for release
│   │       │   │
│   │       │   ├── gdpr/
│   │       │   │   ├── mod.rs
│   │       │   │   ├── export.rs           ← encrypted data export
│   │       │   │   └── erasure.rs          ← crypto shredding + account delete
│   │       │   │
│   │       │   └── ops/
│   │       │       ├── mod.rs
│   │       │       ├── reviews.rs          ← list/view manual review cases
│   │       │       └── decision.rs         ← dual-signature release decision
│   │       │
│   │       ├── services/                   ← business logic (handlers stay thin)
│   │       │   ├── mod.rs
│   │       │   ├── auth_service.rs
│   │       │   ├── vault_service.rs
│   │       │   ├── policy_service.rs
│   │       │   ├── claim_service.rs
│   │       │   ├── release_service.rs      ← m-of-n eval, release record creation
│   │       │   ├── invite_service.rs
│   │       │   ├── file_service.rs         ← B2 presign/confirm logic
│   │       │   └── audit_service.rs        ← audit entry write + chain head update
│   │       │
│   │       ├── db/
│   │       │   ├── mod.rs
│   │       │   ├── pool.rs                 ← sqlx PgPool init
│   │       │   └── queries/                ← compile-time checked SQL per domain
│   │       │       ├── mod.rs
│   │       │       ├── auth.rs
│   │       │       ├── vault.rs
│   │       │       ├── policy.rs
│   │       │       ├── claim.rs
│   │       │       ├── audit.rs
│   │       │       └── ops.rs
│   │       │
│   │       ├── signing/
│   │       │   ├── mod.rs
│   │       │   └── openbao.rs              ← OpenBao Transit sign/verify client
│   │       │
│   │       ├── storage/
│   │       │   ├── mod.rs
│   │       │   └── b2.rs                   ← Backblaze B2 client (presign, head, delete)
│   │       │
│   │       ├── notifications/
│   │       │   ├── mod.rs
│   │       │   └── brevo.rs                ← Brevo API template dispatch
│   │       │
│   │       └── tests/
│   │           ├── integration/
│   │           │   ├── mod.rs
│   │           │   ├── auth_test.rs
│   │           │   ├── vault_test.rs
│   │           │   ├── policy_test.rs
│   │           │   ├── claim_test.rs
│   │           │   ├── release_test.rs
│   │           │   └── audit_test.rs
│   │           └── security/
│   │               ├── mod.rs
│   │               ├── replay_test.rs
│   │               ├── clock_skew_test.rs
│   │               ├── signature_forgery_test.rs
│   │               ├── aead_tamper_test.rs
│   │               ├── wrong_recipient_test.rs
│   │               ├── crypto_version_reject_test.rs
│   │               └── no_server_decrypt_test.rs   ← CI invariant: server never decrypts
│   │
│   └── worker/                             ← Phase 4 onwards
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs
│           ├── lib.rs
│           ├── config.rs
│           ├── scheduler.rs                ← tokio-cron-scheduler setup
│           ├── queue.rs                    ← apalis + Redis backend init
│           ├── dlq.rs                      ← dead letter → ops.failed_jobs + Sentry
│           ├── jobs/
│           │   ├── mod.rs
│           │   ├── heartbeat_eval.rs       ← active→pending→investigating transitions
│           │   ├── notify_owner.rs         ← reminder email dispatch
│           │   ├── notify_beneficiary.rs
│           │   ├── notify_approver.rs
│           │   ├── release_eval.rs         ← m-of-n check → create release record
│           │   ├── conflict_check.rs       ← detect overlapping claims
│           │   ├── envelope_deliver.rs     ← post-hold delivery
│           │   ├── audit_anchor.rs         ← daily B2 anchor + OpenBao sign
│           │   ├── crypto_migration_scan.rs← weekly: notify owners with stale crypto
│           │   └── backup_verify.rs        ← confirm nightly backup succeeded
│           └── tests/
│               ├── heartbeat_eval_test.rs
│               ├── cadence_grace_test.rs   ← all 4 cadence/grace period rules
│               └── release_eval_test.rs
│
├── migrations/                             ← sqlx .sql files, applied in order
│   ├── 0001_create_schemas.sql             ← auth_ext, vault, inheritance, audit, ops, notify
│   ├── 0002_auth_ext_tables.sql            ← persons, person_user_links, opaque_records,
│   │                                          devices, mfa_factors, stepup_challenges
│   ├── 0003_vault_tables.sql               ← items, shares
│   ├── 0004_inheritance_tables.sql         ← policies, heartbeats, claims,
│   │                                          claim_attachments, attestations, release_records
│   ├── 0005_audit_tables.sql               ← audit.events (append-only)
│   ├── 0006_ops_tables.sql                 ← conflict_records, manual_reviews, failed_jobs
│   ├── 0007_notify_tables.sql              ← invites, notification_log
│   ├── 0008_rls_policies.sql               ← deny-by-default RLS on ALL tables
│   ├── 0009_indexes.sql                    ← all performance indexes
│   └── 0010_triggers.sql                   ← updated_at auto-update + policy state transition
│                                              enforcement trigger
│
├── infra/
│   ├── Dockerfile                          ← multi-stage: builder → distroless runtime
│   │                                          all image digests pinned
│   ├── docker-compose.yml                  ← local dev: api, worker, redis, openbao
│   ├── docker-compose.staging.yml          ← staging profile overrides (same Hetzner box)
│   │
│   ├── caddy/
│   │   └── Caddyfile                       ← reverse proxy + auto TLS (Let's Encrypt)
│   │
│   ├── openbao/
│   │   ├── config.hcl                      ← file backend, mlock=true, 127.0.0.1:8200 only
│   │   ├── init-transit.sh                 ← one-time: enable transit + KV, create signing key
│   │   ├── policy-api.hcl                  ← tl-api policy: transit/sign + kv/read only
│   │   └── UNSEAL.md                       ← ✍️ written Phase 0.4 · manual unseal SOP
│   │
│   └── scripts/
│       ├── backup.sh                       ← pg_dump → encrypt → B2 upload
│       ├── restore-test.sh                 ← weekly: restore to ephemeral container, verify
│       └── healthcheck.sh
│
├── rules/                                  ← read before every PR
│   ├── security.rules.md
│   ├── crypto.rules.md
│   ├── api.rules.md
│   ├── db.rules.md
│   ├── code.rules.md
│   ├── git.rules.md
│   └── infra.rules.md
│
├── docs/
│   ├── runbooks/                           ← ✍️ written during relevant phase, not upfront
│   │   ├── incident-response.md            ← Phase 8
│   │   ├── conflict-resolution.md          ← Phase 6
│   │   ├── manual-review-ops.md            ← Phase 8
│   │   └── backup-restore.md               ← Phase 10
│   │
│   └── adr/                                ← Architecture Decision Records
│       ├── 001-rust-axum-stack.md          ← Phase 0
│       ├── 002-openbao-kms-and-secrets.md  ← Phase 0 · OpenBao for both KMS + KV
│       ├── 003-opaque-over-srp.md          ← Phase 1
│       ├── 004-hybrid-kem-design.md        ← Phase 2
│       └── 005-supabase-schema-split.md    ← Phase 0
│
├── output/                                 ← generated project docs (not shipped)
│   ├── project_detail.md
│   └── DIRECTORY_STRUCTURE.md
│
└── .github/
    ├── CODEOWNERS
    ├── pull_request_template.md
    └── workflows/
        ├── ci.yml                          ← fmt + clippy + test + deny + audit (on every PR)
        ├── security-scan.yml               ← trivy + semgrep + gitleaks (weekly)
        └── deploy.yml                      ← tag v* → build → push → Hetzner SSH deploy
```

---

## File Purpose Quick Reference

### Crate Responsibilities

| Crate | Purpose |
|---|---|
| `shared-types` | Domain models, error types, schema/crypto version constants — no business logic |
| `crypto-core` | All crypto primitives: AEAD, KDF, OPAQUE, hybrid KEM, Ed25519, JCS, memory protection |
| `api` | HTTP server: routing, middleware, handlers, services, DB queries, external client wrappers |
| `worker` | Background jobs: heartbeat eval, notifications, release eval, conflict check, audit anchors |

### Migration Order Logic

| Migration | Creates |
|---|---|
| 0001 | 6 schemas |
| 0002 | auth_ext tables (persons, devices, OPAQUE, MFA) |
| 0003 | vault tables (items, shares) |
| 0004 | inheritance tables (policies, heartbeats, claims, attestations, release_records) |
| 0005 | audit.events (append-only) |
| 0006 | ops tables (conflicts, reviews, failed_jobs) |
| 0007 | notify tables (invites, notification_log) |
| 0008 | RLS deny-by-default on all tables |
| 0009 | Performance indexes |
| 0010 | Triggers (updated_at + policy state machine enforcement) |

### Documents Written During Development

| File | Written in Phase | Triggered by |
|---|---|---|
| `infra/openbao/UNSEAL.md` | Phase 0.4 | First OpenBao init |
| `docs/adr/001–005` | Phases 0–2 | Each architecture decision finalized |
| `docs/runbooks/conflict-resolution.md` | Phase 6 | Release + conflict logic complete |
| `docs/runbooks/incident-response.md` | Phase 8 | Ops console complete |
| `docs/runbooks/manual-review-ops.md` | Phase 8 | Ops console complete |
| `docs/runbooks/backup-restore.md` | Phase 10 | Backup system complete |

---

## Key Rules

- **`unsafe` in `api` and `worker` crates is denied** — only allowed in `crypto-core` with mandatory `// SAFETY:` comment per block
- **`api` crate never imports decrypt functions from `crypto-core`** — enforced by CI security test
- **No `.env` file ships in Docker image** — all secrets injected from OpenBao KV at runtime
- **All Docker base image digests are pinned** — never use `latest` tags
- **`output/` is for generated project docs only** — never shipped to production
