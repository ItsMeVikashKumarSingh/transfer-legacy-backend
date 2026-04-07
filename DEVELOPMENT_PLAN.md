# Transfer Legacy — Complete Production Development Plan

> **Project:** Transfer Legacy (Digital Inheritance Vault)
> **Stack:** Rust (Axum + Tokio) · PostgreSQL (Supabase) · Redis · Backblaze B2 ·
>             OpenBao · Hetzner CX22 · GitHub Actions
> **Reference document:** `project_detail.md` (canonical spec — READ ONLY)
> **Approach:** Production-ready from Phase 0. No throwaway code. No fallback data.

---

## Tech Stack Reference

| Layer | Choice | Reason |
|---|---|---|
| Language | Rust (stable, pinned) | Memory safety, zero-cost abstractions, no GC pauses |
| HTTP | Axum + Tokio | Async, ergonomic, tower middleware ecosystem |
| Database | PostgreSQL via Supabase | Managed, EU region, PITR, RLS support |
| ORM/Query | sqlx (compile-time SQL) | No runtime SQL string building |
| Auth PAKE | opaque-ke v4 (Ristretto255) | OPAQUE3 — server never sees password-equivalent (§5.1) |
| Symmetric crypto | dryoc (XChaCha20-Poly1305) | Libsodium bindings in Rust, protected memory API |
| PQ KEM | pqcrypto-kyber (Kyber-768) | ML-KEM-768, NIST PQC standard (§4.7) |
| KDF | argon2 crate | Argon2id, configurable per-device params (§4.4) |
| Signing (server) | OpenBao Transit (Ed25519) | Self-hosted, mlock, never user-secret adjacent (§23.7) |
| Config/Secrets | OpenBao KV | Runtime injection, no .env in production |
| File storage | Backblaze B2 | S3-compatible, zero egress, versioning (§23.5) |
| Background jobs | apalis + Redis | Idempotent, observable, DLQ support |
| Cron | tokio-cron-scheduler | In-process, no external dependency |
| Email | Brevo API (template IDs) | Cost-effective, later migrates to AWS SES |
| Error tracking | Sentry (with scrub hook) | No PII/secrets in events |
| Analytics | PostHog (server-side) | Product analytics, no user secret data |
| Metrics | metrics + Prometheus | Internal /metrics, Grafana Cloud dashboards |
| CI/CD | GitHub Actions | fmt, clippy, test, audit, deny, deploy |
| Reverse proxy | Caddy | Auto TLS, minimal config |
| Container | Docker (distroless runtime) | Smallest attack surface |

---

## Architecture — Crate Layout

```
transfer-legacy/          ← Cargo workspace
├── crates/
│   ├── api/              ← Axum HTTP server
│   ├── worker/           ← apalis jobs + cron
│   ├── crypto-core/      ← all crypto (native + WASM target)
│   └── shared-types/     ← domain models, error types, schema versions
├── migrations/           ← sqlx .sql files
├── infra/                ← Docker, Caddy, OpenBao, scripts
├── rules/                ← 7 rules files
├── docs/                 ← runbooks + ADRs (written per phase)
└── .github/workflows/    ← CI, security scan, deploy
```

---

## Master Checklist

Copy this into your project tracker. Check off as phases complete.

### Phase 0 — Foundation
- [ ] 0.1 Monorepo + Cargo workspace
- [ ] 0.2 Hetzner CX22 provisioned + hardened
- [ ] 0.3 Supabase project + 6 schemas + RLS deny-by-default
- [ ] 0.4 OpenBao Transit engine + Ed25519 signing key + UNSEAL.md written
- [ ] 0.5 OpenBao KV + all secrets populated
- [ ] 0.6 Backblaze B2 buckets (files, audit-anchors, db-backups)
- [ ] 0.7 Redis 7 deployed (internal only, requirepass)
- [ ] 0.8 GitHub Actions: ci.yml + security-scan.yml + deploy.yml
- [ ] 0.9 Base Axum server: middleware stack + /health + /v1/server-capabilities

### Phase 1 — Auth & Identity
- [ ] 1.1 Supabase Auth + Brevo SMTP configured
- [ ] 1.2 OPAQUE register/login endpoints (§5.1, §5.2)
- [ ] 1.3 App-layer AEAD transport + replay/skew protection (§4.5)
- [ ] 1.4 Device key registration + Ed25519 device sig verify (§4.9)
- [ ] 1.5 TOTP + WebAuthn MFA + step-up challenges (§5.3)
- [ ] 1.6 person_id / user_id identity model (§2.4)
- [ ] 1.7 Session management (logout, refresh, password reset)

### Phase 2 — Vault
- [ ] 2.1 vault.items + vault.shares schema + RLS
- [ ] 2.2 Item CRUD endpoints (ciphertext only — no decrypt path)
- [ ] 2.3 Share/pre-wrap endpoints + grant_sig verification (§4.7, §4.12)
- [ ] 2.4 Server capabilities endpoint
- [ ] 2.5 Crypto version gate (reject deprecated versions) (§4.15)
- [ ] 2.6 Crypto version migration path (re-KDF on login if params stale) (§4.4)

### Phase 3 — Policy & Heartbeat
- [ ] 3.1 inheritance.policies schema (all 8 statuses) + status transition trigger
- [ ] 3.2 Policy create/update endpoints + step-up (§23.1)
- [ ] 3.3 Heartbeat endpoint + device sig verify + policy revival (§23.2)
- [ ] 3.4 Invite system + HMAC claim tokens (§23.3)

### Phase 4 — Workers & Notifications
- [ ] 4.1 apalis + Redis worker infrastructure + DLQ
- [ ] 4.2 Heartbeat evaluation worker (all 4 cadence/grace rules) (§21)
- [ ] 4.3 Notification worker: all Brevo templates (§22)
- [ ] 4.4 Daily audit anchor worker → OpenBao sign → B2 upload (§24.2)

### Phase 5 — Claims, Files & Attestations
- [ ] 5.1 claims + claim_attachments + attestations + release_records schema
- [ ] 5.2 Claim initiation + Type A confirmation window (§23.4)
- [ ] 5.3 B2 presign + file confirm + hash verify (§23.5)
- [ ] 5.4 Attestation endpoint + Ed25519 verify + optional Dilithium-2 (§23.6)
- [ ] 5.5 m-of-n evaluation worker (§23.7 preconditionsHere is the complete updated development plan with all fixes applied and a master checklist at the end. Since I cannot write to a file in this turn, copy this directly — it is the final corrected version.

---

```markdown
# Transfer Legacy — Complete Production Development Plan v2

> **Project:** Transfer Legacy (Digital Inheritance Vault)
> **Stack:** Rust (Axum + Tokio) · PostgreSQL (Supabase) · Redis · Backblaze B2
>            · OpenBao · Hetzner CX22 · GitHub Actions
> **Reference document:** `project_detail.md` (canonical spec — READ ONLY)
> **Rules files:** `rules/` directory (7 files — READ BEFORE CODING)
> **Approach:** Production-ready from Phase 0. No throwaway code. No fallbacks.

***

## Tech Stack Reference

| Layer | Choice | Reason |
|-------|--------|--------|
| Language | Rust (stable, pinned) | Memory safety, zero-cost crypto, WASM compat |
| HTTP | Axum + Tokio | Async, tower middleware, ergonomic |
| DB driver | sqlx (compile-time checked) | No ORM magic, real SQL, migrations |
| Database | PostgreSQL via Supabase | Managed, RLS, PITR |
| Cache / Queue | Redis 7 | Rate limiting, OPAQUE state, apalis jobs |
| Background jobs | apalis + tokio-cron-scheduler | Idempotent, Redis-backed, observable |
| Crypto | dryoc + pqcrypto + opaque-ke + argon2 | Audited, WASM-compatible |
| Memory protection | dryoc::protected + zeroize | mlock, zeroise on drop |
| Signing / KMS | OpenBao Transit (Ed25519) | Self-hosted, no cloud vendor lock |
| Secret management | OpenBao KV | Zero plaintext in env/Docker |
| File storage | Backblaze B2 | Zero egress, S3-compatible, versioning |
| Email | Brevo (SMTP + API templates) | Cost-effective, later → AWS SES |
| Error tracking | Sentry (PII scrubbed) | Before-send hook strips secrets |
| Analytics | PostHog (server-side) | No PII events |
| Reverse proxy | Caddy | Auto TLS, simple config |
| CI/CD | GitHub Actions | fmt, clippy, test, audit, deploy |

***

## Architecture Overview

```
transfer-legacy/
├── crates/
│   ├── api/            ← Axum HTTP server
│   ├── worker/         ← apalis jobs + cron scheduler
│   ├── crypto-core/    ← dryoc + pqcrypto + opaque-ke (native + WASM)
│   └── shared-types/   ← domain models, errors, schema versions
├── migrations/         ← sqlx .sql files
├── infra/              ← Dockerfile, docker-compose, OpenBao
├── rules/              ← 7 rules files
├── docs/               ← runbooks (written per phase), ADRs
└── .github/workflows/  ← ci.yml, security-scan.yml, deploy.yml
```

***

## Phase 0 — Foundation & Infrastructure
> **Goal:** Everything runnable locally and in CI before a single business
>           logic line is written.
> **Duration:** ~1 week

### 0.1 Repository Setup

- [ ] Create GitHub repo `transfer-legacy`, branch protection on `main`
      (require PR, require CI green, no force push)
- [ ] Init Cargo workspace: members `api`, `worker`, `crypto-core`,
      `shared-types`
- [ ] `.cargo/config.toml`: `resolver = "2"`, deny `unsafe` in `api` + `worker`
      (allow in `crypto-core` with mandatory `// SAFETY:` comment per block)
- [ ] `rust-toolchain.toml`: pin stable Rust (e.g. `1.77+`) — upgrade
      deliberately, never `latest`
- [ ] `.clippy.toml`: `warn-on-all-lints = true`
- [ ] `deny.toml`: cargo-deny, deny GPL/AGPL licenses, fail on RUSTSEC advisories
- [ ] `rustfmt.toml`: consistent formatting
- [ ] `pull_request_template.md`: checklist (tests pass, no secrets in logs,
      audit entry added, rules reviewed)
- [ ] Write `docs/adr/001-rust-axum-stack.md` — decision rationale

### 0.2 Hetzner Server

- [ ] Provision Hetzner CX22 (2 vCPU / 4 GB / 40 GB SSD) — EU datacenter
      (Nuremberg or Helsinki — GDPR boundary)
- [ ] Harden: disable root SSH, key-only auth, UFW (allow 22, 80, 443 only —
      all internal services bind 127.0.0.1)
- [ ] Install Docker + Docker Compose v2, create non-root `deploy` user
- [ ] Set up Caddy as reverse proxy with auto TLS (Let's Encrypt)
- [ ] Configure `logrotate` — 30-day max retention, no secrets in system logs

### 0.3 Supabase Setup
> Ref: `§3.1 System Components` of project_detail.md

- [ ] Create Supabase project — EU region (`eu-central-1`)
- [ ] Configure custom SMTP → Brevo (Settings → Auth → SMTP)
      Removes 2 email/hr dev limit
- [ ] Create 6 PostgreSQL schemas via migration `0001_create_schemas.sql`:
      `auth_ext`, `vault`, `inheritance`, `audit`, `ops`, `notify`
      - `auth_ext` — device keys, OPAQUE records, MFA, step-up challenges
      - `vault` — items, shares
      - `inheritance` — policies, heartbeats, claims, attestations,
                        release_records
      - `audit` — append-only event chain (write-once RLS)
      - `ops` — conflict records, manual reviews, failed_jobs
      - `notify` — invites, notification_log
- [ ] Enable RLS on ALL tables — deny by default, service role bypasses
- [ ] Enable `pg_audit` extension (DDL/DML logging)
- [ ] Enable `pg_crypto` extension (invite token HMAC)
- [ ] Set `max_connections = 50` (Supabase free: 60 — reserve headroom)
- [ ] Activate Supabase Pro plan before go-live (PITR backups)

### 0.4 OpenBao Setup (Signing)
> Ref: `§3.2 High-Level Flow` + `§4.8 Canonicalization & Signatures`

- [ ] Deploy OpenBao via Docker on Hetzner
      (`docker pull openbao/openbao:latest`)
- [ ] `config.hcl`: file backend `/vault/data`, `mlock = true`,
      listener on `127.0.0.1:8200` — never internet-exposed
- [ ] Init vault; store unseal keys in separate encrypted locations
      (NOT on Hetzner server)
- [ ] Enable Transit engine: `bao secrets enable transit`
- [ ] Create Ed25519 signing key: `bao write transit/keys/tl-signing type=ed25519`
      Used for: `release_record` hash signing (`§23.7`) and daily audit
      anchors (`§24.2`)
- [ ] Policy `tl-api`: allows only `transit/sign/tl-signing` and
      `transit/verify/tl-signing`
- [ ] Generate AppRole token for Rust API with `tl-api` policy
- [ ] Write `infra/openbao/init-transit.sh` — one-time setup script
- [ ] ✍️ Write `infra/openbao/UNSEAL.md` — manual unseal SOP
- [ ] Write `docs/adr/002-openbao-over-aws-kms.md`

### 0.5 OpenBao KV Setup (Config Management)

- [ ] Enable OpenBao KV (v2) on the Hetzner host
      (separate from API container)
- [ ] Create a KV namespace/path for `transfer-legacy`
- [ ] Populate secrets at that path:
      ```
      SUPABASE_URL
      SUPABASE_SECRET_KEY
      SUPABASE_PUBLISHABLE_KEY
      REDIS_URL
      OPENBAO_ADDR                   (http://127.0.0.1:8200)
      OPENBAO_TOKEN
      BREVO_API_KEY
      BREVO_SMTP_PASSWORD
      BACKBLAZE_B2_KEY_ID
      BACKBLAZE_B2_APP_KEY
      BACKBLAZE_B2_BUCKET_NAME
      BACKBLAZE_B2_ENDPOINT_URL
      SENTRY_DSN
      POSTHOG_API_KEY
      SERVER_HMAC_SECRET             (invite token HMAC — §23.3)
      SERVER_AEAD_KEY                (transport AEAD key — §4.5)
      JWT_SECRET
      ```
- [ ] Install OpenBao CLI (`bao`) on Hetzner or use the HTTP API to manage KV
- [ ] API loads secrets from OpenBao KV at startup (no `.env` in production)
- [ ] RULE: Zero plaintext secrets in `docker-compose.yml`, `Dockerfile`,
      or Git. Ever.

### 0.6 Backblaze B2 Setup
> Ref: `§23.5 File Presign & Upload` of project_detail.md

- [ ] Create B2 bucket `tl-user-files-prod` — encrypted user documents
- [ ] Create B2 bucket `tl-audit-anchors-prod` — daily signed anchors (`§24.2`)
- [ ] Create B2 bucket `tl-db-backups-prod` — nightly pg_dump
- [ ] Enable versioning on `tl-user-files-prod` (30-day object retention)
- [ ] API token: `Object Read & Write` on `tl-user-files-prod` only
- [ ] CORS: allow presigned PUT from app domains only (no wildcard)

### 0.7 Redis Setup

- [ ] Deploy Redis 7 via Docker: `requirepass` set, bind `127.0.0.1` only
- [ ] `maxmemory-policy allkeys-lru` — not a persistent store
- [ ] Add `REDIS_URL` to OpenBao KV

### 0.8 GitHub Actions CI Pipeline

- [ ] `ci.yml` — on push/PR to `main` and `develop`:
      - `cargo fmt --check`
      - `cargo clippy --all-targets -- -D warnings`
      - `cargo test --workspace`
      - `cargo deny check`
      - `cargo audit`
- [ ] `security-scan.yml` — weekly:
      - `trivy` image scan (fail on HIGH/CRITICAL CVEs)
      - `semgrep` with security ruleset
      - `gitleaks` secret scan
- [ ] `deploy.yml` — on tag `v*`:
      build release binary → Docker build → SSH push to Hetzner
- [ ] CI gate: ALL checks must pass before merge. No bypass.
- [ ] Pin all Docker image digests in Dockerfile
      (`FROM rust:1.77@sha256:...` not `FROM rust:latest`)

### 0.9 Base Axum Server + Shared Types

- [ ] `crates/shared-types`: `PolicyStatus` enum (all 8 states — see §3.1):
      `Active`, `Pending`, `Investigating`, `ReleaseReady`,
      `ConflictPending`, `ManualReview`, `Released`, `Cancelled`
- [ ] `crates/shared-types`: `AppError` enum → stable RFC 7807 error codes
      (`ERR_AEAD_INTEGRITY`, `ERR_REPLAY_DETECTED`, `ERR_SIGNATURE_INVALID`,
       `ERR_ENVELOPE_RECIPIENT_MISMATCH`, `ERR_CRYPTO_VERSION_UNSUPPORTED`,
       `ERR_DUAL_SIGNATURE_REQUIRED`, etc.)
- [ ] `crates/shared-types`: `CryptoVersion` enum, `SchemaVersion` constants
- [ ] Init `crates/api` with Axum + Tokio
- [ ] Middleware stack (applied in order):
      1. `TraceLayer` — structured request tracing (tracing crate)
      2. `TimeoutLayer` — 30s request timeout
      3. `RequestBodyLimitLayer` — 10 MB max body
      4. `CorsLayer` — strict domain allowlist (no wildcard)
      5. `SecurityHeadersLayer` — HSTS, X-Content-Type-Options,
         X-Frame-Options, CSP
      6. `RequestIdLayer` — inject `X-Request-ID` UUID
      7. `SentryLayer` — propagate request context
- [ ] `GET /health` → `{ "status": "ok", "version": "<git-sha>" }`
- [ ] `GET /v1/server-capabilities` → supported crypto/OPAQUE params (`§3.1`)
- [ ] RFC 7807 global error handler
- [ ] Sentry SDK init with `before_send` hook stripping all secret-adjacent
      field names before transmission
- [ ] PostHog server-side client init
- [ ] Prometheus metrics endpoint on internal port (not public)

### Phase 0 Acceptance Criteria

- [ ] `cargo test --workspace` passes in CI
- [ ] `/health` returns 200 from Hetzner
- [ ] All 6 schemas created, RLS deny-by-default verified on test DB
- [ ] OpenBao Transit: sign test payload → verify → passes
- [ ] OpenBao KV secrets are available to the API at startup (no `.env` in prod)
- [ ] B2: presigned PUT upload test succeeds
- [ ] Sentry receives test event — PII and secret fields absent
- [ ] `PolicyStatus` enum has exactly 8 states in `shared-types`

***

## Phase 1 — Authentication & Identity
> **Goal:** Complete production auth: OPAQUE + device keys + MFA + sessions.
> Ref: `§5.1–5.7`, `§4.9`, `§5.3`, `§2.4` of project_detail.md
> **Duration:** ~3 weeks

### 1.1 Supabase Auth Integration

- [ ] Enable email + password, email confirmation required
- [ ] Brevo SMTP verified end-to-end (verification, reset, welcome)
- [ ] Brevo template IDs created for (ref `§22` notification templates):
      email verification, password reset, welcome post-verification
- [ ] JWT: `access_token` expiry = 1h, `refresh_token` = 7d
- [ ] Configure Supabase Auth rate limits via dashboard

### 1.2 OPAQUE Authentication
> Ref: `§5.1 Registration OPAQUE Full Flow`, `§5.2 Login OPAQUE`

- [ ] Add `opaque-ke` v4.x to `crypto-core` — OPAQUE3 with
      Ristretto255 + HKDF-SHA512
- [ ] Server-side OPAQUE state in Redis (TTL 5 min):
      - `opaque:reg:{session_id}` → `ServerRegistration` state
      - `opaque:login:{session_id}` → `ServerLogin` state
- [ ] Endpoints:
      - `POST /v1/auth/register/init`
        ← returns `OPAQUEmsg2`, `server_nonce`
      - `PUT /v1/auth/register/finish` (AEAD body)
        ← `OPAQUEmsg3`, `public_keys`, `emk_blob`, `argon2_params`
        → stores `pake_record`, `public_keys` (x25519 / ed25519 / kyber768),
          `emk`, `argon2_params` in `auth_ext.opaque_records`
        → returns `user_id`
      - `POST /v1/auth/login/init`
        ← returns `OPAQUEmsg2`, `server_nonce`, `session_params`
      - `POST /v1/auth/login/finish` (AEAD body)
        ← `OPAQUEmsg3`
        → on success: Supabase session + `session_token`,
          `user_public_keys`, `emk_blob`, `argon2_params`
- [ ] **CI-enforced invariant:** no function in `crates/api` may call any
      decrypt function from `crypto-core`. Static analysis test asserts this.
      (`tests/security/no_server_decrypt_test.rs`)
- [ ] Write `docs/adr/003-opaque-over-srp.md` (ref `§5.12`)

### 1.3 App-Layer AEAD Transport
> Ref: `§4.5 App-layer AEAD Transport over TLS`

- [ ] `AeadTransportLayer` Axum extractor:
      - Decrypts request body: XChaCha20-Poly1305 with `SERVER_AEAD_KEY`
      - Verifies `X-Seq` monotonic counter per device (replay — `§4.14`)
      - Verifies timestamp within ±5 minutes (clock skew — `§4.14`)
      - On failure → `ERR_REPLAY_OR_SKEW` or `ERR_AEAD_INTEGRITY`
- [ ] `X-Request-ID` propagated through AEAD responses
- [ ] Idempotency key: `X-Idempotency-Key` header cached in Redis (TTL 24h)
      Required on ALL mutating endpoints

### 1.4 Device Keys
> Ref: `§4.9 Device Keys & WebAuthn`, `§5.2`

- [ ] Schema `auth_ext.devices`:
      `device_id`, `user_id`, `ed25519_pubkey`, `device_meta`,
      `created_at`, `last_seen_at`
- [ ] `POST /v1/devices/register` (AEAD):
      - Verifies `device_sig` = Ed25519 sign of
        `sha256(JCS({ device_id, user_id, ts }))` with device private key
      - Max 10 devices per user — 11th → `device_limit_exceeded`
- [ ] `GET /v1/devices` — list user's registered devices
- [ ] `DELETE /v1/devices/{device_id}` — step-up auth required

### 1.5 MFA / Step-up
> Ref: `§5.3 MFA Step-up Authentication`

- [ ] TOTP (`totp-rs` — RFC 6238, SHA1 HMAC, 30s window):
      - `POST /v1/auth/mfa/totp/enroll` → QR URI + backup codes
      - `POST /v1/auth/mfa/totp/verify`
      - TOTP secret encrypted in `auth_ext.mfa_factors`
- [ ] WebAuthn / Passkeys (`webauthn-rs`):
      - `POST /v1/auth/mfa/webauthn/register/start` + `/finish`
      - `POST /v1/auth/mfa/webauthn/authenticate/start` + `/finish`
- [ ] Step-up challenges for sensitive actions (schema:
      `auth_ext.stepup_challenges`):
      - Adding / removing beneficiaries or approvers
      - Changing heartbeat cadence
      - Cancelling policy during `pending` / `investigating`
      - Device revocation

### 1.6 Person / User Identity Model
> Ref: `§2.4 Identity vs Accounts (person_id vs user_id)`

- [ ] Schema `auth_ext.persons`:
      `person_id`, `enc_legal_name` (AEAD ciphertext), `enc_email`,
      `kyc_status`, `created_at`
- [ ] Schema `auth_ext.person_user_links`:
      `person_id`, `user_id`, `linked_at`
      — one person → many users over time
- [ ] On registration: auto-create `person` + link to `user_id`
- [ ] On invite acceptance: link existing or new `person_id` to policy role

### 1.7 Session Management

- [ ] Sessions backed by Supabase Auth JWTs
- [ ] `POST /v1/auth/logout` — revoke session, clear Redis session data
- [ ] `POST /v1/auth/refresh` — wrap Supabase token refresh
- [ ] `POST /v1/auth/password/reset/request` — Brevo email via Supabase Auth
- [ ] `POST /v1/auth/password/reset/confirm` — validate token, update auth

### Phase 1 Acceptance Criteria

- [ ] OPAQUE register → login round-trip passes integration test
- [ ] `pake_record` stored — CI asserts server cannot call decrypt
- [ ] Device signature verification fails on tampered payload
- [ ] TOTP enroll → verify cycle passes
- [ ] WebAuthn challenge → assertion passes
- [ ] Step-up blocks unauthorized sensitive action
- [ ] RFC 7807 error returned for all auth failures (no user enumeration)
- [ ] Rate limiting: >10 login attempts/min → 429

***

## Phase 2 — Vault (Secret Storage)
> **Goal:** Owners can store encrypted items and pre-wrap keys for beneficiaries.
>           Includes crypto version migration path.
> Ref: `§7.2`, `§4.2`, `§4.6`, `§4.7`, `§4.11`, `§4.12`, Sequence 2
> **Duration:** ~3 weeks

### 2.1 Database Schema

- [ ] `vault.items`:
      `item_id`, `owner_id`, `ciphertext`, `nonce`, `enc_item_key_owner`,
      `crypto_version`, `schema_version`, `item_type` ENUM
      (`password`, `wallet_credential`, `note`, `file_reference`,
       `identity_document`, `property_document`, `crypto_seed`),
      `created_at`, `updated_at`
- [ ] `vault.shares`:
      `share_id`, `item_id`, `owner_id`, `grantee_id`,
      `envelope` JSONB (hybrid-wrap-v1 schema), `grant_sig`, `created_at`
      - RLS: owner reads own shares; grantee reads their own
      - Append-only: INSERT + soft-delete flag only (no UPDATE)
- [ ] Index: `vault.shares(grantee_id, item_id)` — release-time fetch

### 2.2 Item CRUD Endpoints

- [ ] `POST /v1/vault/items` (AEAD body):
      `{ ciphertext, nonce, enc_item_key_owner, meta,
         crypto_version, schema_version }`
      - Validate JSON schema against `§4.6` Item format
      - Server MUST NOT attempt to decrypt `ciphertext` (CI invariant)
      - Returns `item_id`
- [ ] `GET /v1/vault/items` — metadata list (no ciphertext in list view)
- [ ] `GET /v1/vault/items/{item_id}` — full record for owner
- [ ] `PUT /v1/vault/items/{item_id}` — update (step-up for type change)
- [ ] `DELETE /v1/vault/items/{item_id}` — step-up required;
      soft-delete item + all shares; audit entry `item_deleted`

### 2.3 Share / Pre-wrap Endpoints
> Ref: `§4.7 Hybrid KEM Envelope`, `§4.12 Client Pseudocode`, Sequence 2

- [ ] `POST /v1/vault/shares` (AEAD body):
      `{ envelope (ShareEnvelope JSON), grant_sig }`
      - Verify `grant_sig` = Ed25519 sign of `sha256(JCS(envelope))`
        using owner's registered `ed25519_pubkey` (`§4.8`)
      - Validate `ShareEnvelope` JSON schema (`§4.11`)
      - Store — never decrypt
- [ ] `GET /v1/vault/shares` — owner sees all shares they created
- [ ] `DELETE /v1/vault/shares/{share_id}` — revoke (owner only, step-up)
- [ ] `crypto_version` gate: reject unsupported version with
      `ERR_CRYPTO_VERSION_UNSUPPORTED`
- [ ] Nonce uniqueness check: alert metric `nonce_reuse_detected_total`
      on collision (never silently allow)

### 2.4 Crypto Version Migration Path
> Ref: `§4.15 Crypto Version Gates`, `§4.2 Primitive Version Matrix`
> **This was missing from v1 of the plan — required fix**

- [ ] On login response: include `migration_required: bool` flag if stored
      `argon2_params` are below current minimum parameters
- [ ] `PUT /v1/vault/crypto-migrate` (AEAD body):
      `{ new_emk_blob, new_argon2_params, re_encrypted_items[] }`
      - Client re-derives KEK with new params, re-encrypts EMK, uploads
      - Server atomically replaces `emk` + `argon2_params`
      - Audit entry: `crypto_params_migrated`
- [ ] Worker: weekly scan for items with deprecated `crypto_version`
      → notify owner to re-encrypt via client
- [ ] Every item/share/envelope must include `crypto_version` and
      `schema_version` — reject on read if unsupported
- [ ] Write `docs/adr/004-hybrid-kem-design.md` (ref `§4.7`)

### 2.5 Server Capability Endpoint

- [ ] `GET /v1/server-capabilities` (public, no auth):
      ```json
      {
        "supported_crypto_versions": ["aead-xchacha20p1305-v1"],
        "deprecated_crypto_versions": [],
        "opaque_params": { "suite": "ristretto255", "hash": "sha512" },
        "pq_supported": true,
        "kyber_version": "kyber768-v1",
        "schema_version": 1
      }
      ```

### Phase 2 Acceptance Criteria

- [ ] Create item → fetch → ciphertext unchanged (integration test)
- [ ] `grant_sig` verification fails on tampered envelope
- [ ] Wrong `crypto_version` → `ERR_CRYPTO_VERSION_UNSUPPORTED`
- [ ] Nonce collision → metric `nonce_reuse_detected_total` increments
- [ ] Server decrypt attempt → `ServerNotAllowedToDecrypt` error (CI test)
- [ ] Migration: low Argon2 params → `migration_required: true` in login
- [ ] Migration endpoint atomically updates EMK + params

***

## Phase 3 — Inheritance Policy & Heartbeat
> **Goal:** Owners configure policies, invite participants, keep alive via
>           heartbeat. Full 8-state machine enforced at DB level.
> Ref: `§23.1`, `§23.2`, `§23.3`, `§2.1 FR §3-4`, `§2.3`, `§21`
> **Duration:** ~3 weeks

### 3.1 Policy Schema

- [ ] `inheritance.policies`:
      `policy_id`, `owner_id`, `policy_type` ENUM(`direct_transfer`,`m_of_n`),
      `cadence` ENUM(`1w`,`15d`,`1m`,`3m`), `m_of_n` JSONB,
      `beneficiaries` JSONB, `approvers` JSONB,
      `release_conditions` JSONB,
      `status` ENUM — **exactly 8 states**:
        `active`, `pending`, `investigating`, `release_ready`,
        `conflict_pending`, `manual_review`, `released`, `cancelled`
      `last_heartbeat_at`, `pending_at`, `grace_deadline`,
      `conflict_hold_until`, `audit_head_hash`,
      `created_at`, `crypto_version`, `schema_version`
- [ ] **PostgreSQL trigger** on `inheritance.policies`: enforce valid
      transitions only — invalid transition raises exception:
      ```
      active      → pending (worker only)
      pending     → active (heartbeat revival)
      pending     → investigating (worker only)
      investigating → release_ready (worker only)
      investigating → conflict_pending (worker)
      release_ready → released (worker, post hold)
      release_ready → conflict_pending (worker, conflict detected)
      conflict_pending → manual_review (worker, hold expired unresolved)
      manual_review → released (ops dual-sig)
      manual_review → cancelled (ops or legal order)
      active/pending → cancelled (owner step-up)
      ```
- [ ] `inheritance.heartbeats`:
      `heartbeat_id`, `policy_id`, `device_id`, `device_sig`, `ts`,
      `received_at`
- [ ] Write `docs/adr/005-supabase-schema-split.md`

### 3.2 Policy Endpoints

- [ ] `PUT /v1/inheritance/policy` (AEAD, step-up required):
      - Validates `m_of_n` (m ≤ n, m ≥ 1), cadence, beneficiary list
      - Calculates `pending_at` = `last_heartbeat_at + cadence_days`
      - Calculates `grace_deadline` per `§21`:
        - 1w → `pending_at + 28d` (4× cadence)
        - 15d → `pending_at + 45d` (3× cadence)
        - 1m → `pending_at + 90d` (3× cadence)
        - 3m → `pending_at + 90d` (1× cadence)
      - Audit entry: `policy_created` with `payload_hash = sha256(JCS(policy))`

### 3.3 Heartbeat Endpoint
> Ref: `§23.2`

- [ ] `POST /v1/inheritance/heartbeat` (AEAD):
      `{ policy_id, ts, device_id, device_sig }`
      - Verify `device_sig` = Ed25519 sign of
        `sha256(JCS({ policy_id, ts, device_id }))`
      - Update `policies.last_heartbeat_at = ts`
      - Recalculate `pending_at` and `grace_deadline`
      - If `status` = `pending` or `investigating` → revert to `active`
        (policy revival)
      - Audit entry: `heartbeat_received`
      - Returns: updated `pending_at`, `grace_deadline`, new `status`

### 3.4 Invite System
> Ref: `§23.3`, `§5.7`

- [ ] `POST /v1/inheritance/policy/{policy_id}/invite` (AEAD, step-up):
      - `invite_id` = UUID
      - `claim_token = HMAC-SHA256(SERVER_HMAC_SECRET,
                                   concat(invite_id, email, expires_at))`
      - Store in `notify.invites`: `invite_id`, `policy_id`, `email`,
        `claim_token_hmac`, `expires_at`, `used = false`
      - Send invite email via Brevo (template `BENEFICIARY_INVITE`)
      - Audit entry: `invite_created`
- [ ] `POST /v1/inheritance/claim-token/consume`:
      `{ invite_id, claim_token, public_keys }`
      - Verify HMAC, `used = false`, `expires_at` not passed
      - PostgreSQL advisory lock: atomically set `used = true`
        (concurrent consumption safe — only one wins)
      - Link `person_id` to policy role
      - Audit entry: `invite_consumed`

### Phase 3 Acceptance Criteria

- [ ] Heartbeat updates deadlines correctly for all 4 cadences (unit test)
- [ ] Policy revival: heartbeat during `investigating` → `active`
- [ ] Invalid status transition rejected by DB trigger
- [ ] Concurrent claim-token: only one succeeds (parallel request test)
- [ ] `device_sig` forgery fails on tampered heartbeat
- [ ] Grace deadlines correct per `§21` exact mapping (4 unit tests)

***

## Phase 4 — Background Workers & Notifications
> **Goal:** Automated state machine transitions, reliable job processing,
>           email dispatch, daily audit anchoring.
> Ref: `§21 Timers`, `§22 Notification Templates`, `§24.2`
> **Duration:** ~2 weeks

### 4.1 Worker Infrastructure

- [ ] `crates/worker` — apalis with Redis backend
- [ ] `tokio-cron-scheduler` for periodic tasks
- [ ] All jobs are **idempotent** — safe to retry
- [ ] DLQ: failed jobs after 3 retries → `ops.failed_jobs` + Sentry alert
- [ ] Job types: `HeartbeatEvalJob`, `NotifyOwnerJob`,
      `NotifyBeneficiaryJob`, `NotifyApproverJob`,
      `AuditAnchorJob`, `ConflictCheckJob`, `CryptoMigrationScanJob`

### 4.2 Heartbeat Evaluation Worker

- [ ] Schedule: every hour via cron
- [ ] Query: `status = active` AND `now() >= pending_at`
      → set `status = pending`, enqueue `NotifyOwnerJob`
- [ ] Query: `status = pending` AND `now() >= grace_deadline`
      → set `status = investigating`,
        enqueue `NotifyBeneficiaryJob` + `NotifyApproverJob`
- [ ] All transitions write audit entry with `prev_hash` chaining (`§4.8`)
- [ ] Reminder schedule per `§21`:
      - 1w: remind at `pending_at - 3d`, `pending_at - 1d`,
             daily in last week, 2× on last day
      - 15d / 1m / 3m: proportional per `§21`

### 4.3 Notification Worker
> Ref: `§22 Notification Templates`

- [ ] All transactional emails via Brevo API (template IDs, not raw SMTP)
- [ ] Brevo template IDs (created in Brevo dashboard, referenced by ID
      in code — never hardcoded content):
      - `OWNER_REMINDER_EARLY` — 3d before pending
      - `OWNER_REMINDER_URGENT` — 1d before pending
      - `OWNER_REMINDER_DAILY` — daily during grace
      - `BENEFICIARY_CLAIM_AVAILABLE` — after `investigating`
      - `APPROVER_ATTESTATION_REQUEST` — after `investigating`
      - `CONFLICT_HOLD_NOTICE` — 48h hold placed
      - `RELEASE_READY` — after hold expires
- [ ] All attempts logged in `notify.notification_log`
- [ ] Failed sends: retry 3× exponential backoff → DLQ → Sentry CRITICAL

### 4.4 Daily Audit Anchor Worker
> Ref: `§24.2 Daily Anchor & Evidence Package`

- [ ] Schedule: every day at 00:05 UTC
- [ ] Compute `head_hash` of all audit entries for the day
- [ ] Construct `anchor_snapshot` JSON (JCS-canonical):
      `{ date, head_hash, count_audit_entries, server_id }`
- [ ] Sign `sha256(JCS(anchor_snapshot))` via OpenBao Transit
- [ ] Upload `anchor_snapshot` + `anchor_sig` to B2:
      `tl-audit-anchors-prod/{YYYY-MM-DD}/anchor.json`
- [ ] Audit entry: `audit_anchor_created`

### Phase 4 Acceptance Criteria

- [ ] `active → pending → investigating` on schedule (time-frozen DB test)
- [ ] All 4 cadence grace rules produce correct values (unit tests per `§21`)
- [ ] Brevo template called with correct variables (mock test)
- [ ] Duplicate run for same policy → single notification (idempotency test)
- [ ] B2 anchor upload succeeds, signature verifies with OpenBao public key
- [ ] Failed job → `ops.failed_jobs` + Sentry alert after 3 retries

***

## Phase 5 — Claims, File Uploads & Attestations
> **Goal:** Beneficiaries initiate claims, upload encrypted evidence,
>           approvers submit signed attestations.
> Ref: `§23.4`, `§23.5`, `§23.6`, `§5.7`, `§2.1 FR §5`
> **Duration:** ~3 weeks

### 5.1 Schema

- [ ] `inheritance.claims`:
      `claim_id`, `policy_id`, `beneficiary_id`,
      `status` ENUM(`pending_docs`,`docs_uploaded`,
                    `attestations_in_progress`,`ready`,
                    `withdrawn`,`rejected`),
      `claim_reason`, `contact_phone`, `sig_ed25519`, `ts`, `created_at`
- [ ] `inheritance.claim_attachments`:
      `attachment_id`, `claim_id`,
      `purpose` ENUM(`death_certificate`,`beneficiary_id_doc`,
                     `supporting_evidence`),
      `r2_key`, `sha256_hex`, `uploaded_at`
- [ ] `inheritance.attestations`:
      `attestation_id`, `policy_id`, `claim_id`, `attestor_id`,
      `decision` ENUM(`approve`,`reject`,`abstain`),
      `notes`, `evidence_refs` JSONB,
      `sig_ed25519`, `sig_pq` (optional Dilithium-2), `ts`, `created_at`
      - UNIQUE constraint: `(claim_id, attestor_id)`
- [ ] `inheritance.release_records`:
      `release_id`, `policy_id`, `claim_id`, `canonical_json`,
      `server_sig`, `conflict_hold_until`, `created_at`

### 5.2 Claim Initiation
> Ref: `§23.4`

- [ ] `POST /v1/inheritance/claim/initiate` (AEAD):
      - Verify `sig_ed25519` = Ed25519 sign of
        `sha256(JCS(body_without_sig))` with beneficiary device key
      - Policy MUST be in `investigating` (or `active`/`pending` for Type A)
      - **Type A:** if `active` or `pending` → open owner confirmation
        window (2–3 days), notify owner; do not immediately release
      - Create claim `status = pending_docs`
      - Audit entry: `claim_initiated`

### 5.3 File Presign & Upload
> Ref: `§23.5`

- [ ] `POST /v1/files/presign` (AEAD):
      `{ claim_id, policy_id, purpose, filename, sha256 }`
      - Generate B2 presigned PUT URL (15-min TTL)
      - B2 key: `{policy_id}/{claim_id}/{purpose}/{uuid}.enc`
      - Store pending attachment (not confirmed yet)
      - Returns: `{ upload_url, r2_key, expires_at }`
- [ ] `POST /v1/files/confirm` (AEAD):
      `{ r2_key, sha256 }`
      - B2 `HeadObject` — verify Content-Length > 0, ETag matches SHA256
      - Mark attachment confirmed; `claim.status = docs_uploaded`
      - Audit entry: `proof_uploaded`
      - RULE: server never proxies file bytes — presign only

### 5.4 Attestation
> Ref: `§23.6`

- [ ] `POST /v1/inheritance/attest` (AEAD):
      - Verify `sig_ed25519` using approver's registered `ed25519_pubkey`
      - If `sig_pq` present (Dilithium-2): verify and persist
      - Insert attestation; recompute `approve_count`
      - Update claim status accordingly
      - Audit entry: `attestation_added`
      - Reject duplicate: UNIQUE `(claim_id, attestor_id)` constraint

### 5.5 m-of-n Evaluation Worker

> Ref: `§23.7 Release Preconditions`

- [ ] Worker: every 15 minutes, check claims in `ready` or
      `attestations_in_progress`
- [ ] Release preconditions — ALL must be true:
      - `policy.status = investigating`
      - `claim.status` = `ready` or `docs_uploaded`
      - `require_death_certificate = true` → attachment present
      - `approve_count >= policy.m_of_n.m`
      - No blocking `reject` from required approvers
      - No manual hold flags in `ops` schema
- [ ] If all satisfied → enqueue `CreateReleaseRecordJob`

### Phase 5 Acceptance Criteria

- [ ] Forged `sig_ed25519` on claim → `ERR_SIGNATURE_INVALID`
- [ ] SHA256 mismatch on file confirm → rejected
- [ ] Duplicate attestation from same approver → DB constraint error
- [ ] 2-of-3 satisfied → release record job enqueued
- [ ] Required approver `reject` blocks release regardless of approve count
- [ ] Type A: claim during `active` → owner notified, window opened

***

## Phase 6 — Release & Envelope Delivery
> **Goal:** Cryptographically signed inheritance release with conflict
>           protection and post-hold envelope delivery.
> Ref: `§23.7`, `§23.8`, `§23.9`, `§4.8`
> **Duration:** ~3 weeks

### 6.1 Release Record Creation
> Ref: `§23.7`

- [ ] Worker constructs canonical `release_record` JSON (JCS):
      ```json
      {
        "release_id": "...", "policy_id": "...", "claim_id": "...",
        "beneficiaries": [...], "items": [...],
        "attestations": [...], "proofs": [...],
        "issued_at": "...", "conflict_hold_until": "...",
        "schema_version": 1, "crypto_version": "v1"
      }
      ```
- [ ] `payload_hash = sha256(JCS(release_record))`
- [ ] Sign via OpenBao Transit: `POST /v1/transit/sign/tl-signing`
- [ ] Store `release_record` + `server_sig` in `inheritance.release_records`
- [ ] `conflict_hold_until = now() + release_conditions.conflict_hold_hours`
      (default 48h)
- [ ] Set `policy.status = release_ready`
- [ ] Notify claimants: Brevo template `CONFLICT_HOLD_NOTICE`
- [ ] Audit entry: `release_record_signed` with `payload_hash` + `prev_hash`
- [ ] ✍️ Write `docs/runbooks/conflict-resolution.md` (ref `§23.9`)

### 6.2 Conflict Detection
> Ref: `§23.9`, `§26.2`

- [ ] Worker: during hold period, detect:
      - Multiple simultaneous claims for same policy (overlapping items)
      - Conflicting attestations across claims
- [ ] If conflict: `policy.status = conflict_pending`,
      create `ops.conflict_records`, Sentry alert + ops email
- [ ] On `conflict_hold_until` passed AND still `conflict_pending`:
      → `policy.status = manual_review`,
        create `ops.manual_reviews` ticket

### 6.3 Envelope Delivery
> Ref: `§23.8`

- [ ] Worker: after `conflict_hold_until` with no conflict:
      - Per beneficiary: gather `vault.shares` where
        `grantee_id = beneficiary_id` AND `item_id IN release_record.items`
      - Build `envelope_batch` JSON (ciphertext only — never plaintext)
      - Set `policy.status = released`
      - Audit entry: `envelopes_delivered`
- [ ] `GET /v1/inheritance/envelopes` (authenticated as beneficiary):
      - Only available if `policy.status = released` AND release exists
      - Returns AEAD-wrapped envelope batch

### 6.4 Evidence Package
> Ref: `§24.2`

- [ ] `GET /v1/inheritance/release/{release_id}/evidence`:
      - Returns AEAD-wrapped: `{ release_record, server_sig,
                                  attestations, proofs, audit_entries,
                                  anchor }`
      - Attachment references are B2 keys (client downloads separately
        via presigned GET)

### Phase 6 Acceptance Criteria

- [ ] Release record canonical JSON produces identical `payload_hash`
      across implementations (test vector — `§4.13`)
- [ ] OpenBao signature verifies with Transit public key
- [ ] Conflict: two simultaneous claims → `conflict_pending`, ops alerted
- [ ] Unresolved after hold → `manual_review` status
- [ ] Envelope batch decryptable by beneficiary keys only
- [ ] `released` → no further heartbeat/claim accepted
- [ ] Evidence package contains all required fields (`§24.2`)

***

## Phase 7 — Audit Chain & Compliance
> **Goal:** Tamper-evident audit trail, GDPR endpoints, data lifecycle.
> Ref: `§4.8`, `§24.1`, `§24.2`
> **Duration:** ~2 weeks

### 7.1 Audit Chain Implementation
> Ref: `§24.1`

- [ ] `audit.events` schema:
      `event_id` (UUID), `policy_id`, `event_type`, `payload_hash`
      (SHA256 of canonical event JSON), `prev_hash`, `actor_id`,
      `ip_hash` (SHA256 of IP — raw IP never stored), `created_at`
- [ ] Append-only RLS: INSERT only, no UPDATE/DELETE for service role
      (separate admin-override role for legal holds only)
- [ ] Chain formula: `hash = sha256(JCS({ event_id, payload_hash,
      prev_hash }))` — verifiable offline
- [ ] `GET /v1/audit/{policy_id}` — ops/admin only, paginated chain
- [ ] `GET /v1/audit/{policy_id}/verify` — returns pass/fail + first
      broken link if any

### 7.2 GDPR Compliance Endpoints

- [ ] `GET /v1/gdpr/export` — authenticated:
      exports all user data as encrypted ciphertext export
      (user decrypts locally — server never exports plaintext)
- [ ] `DELETE /v1/gdpr/erasure` — authenticated + step-up:
      - Delete `emk` + device keys → cryptographic shredding
        (all ciphertext permanently inaccessible)
      - Delete Supabase Auth account
      - Audit entry: `user_data_erased` (retained minimum 7 years —
        GDPR Art. 17(3))
      - B2 files: schedule deletion after 30-day versioning window
- [ ] Raw IP addresses never stored (only `ip_hash`)
- [ ] Audit event retention: minimum 7 years for legal defensibility

### Phase 7 Acceptance Criteria

- [ ] `prev_hash` links correctly across 100 sequential events
- [ ] Tampered audit entry detected by verify endpoint
- [ ] GDPR erasure: `GET /v1/vault/items` returns empty, `emk` gone
- [ ] Daily anchor: B2 object exists per day, signature valid

***

## Phase 8 — Admin, Ops & Manual Review
> **Goal:** Ops can intervene in conflict cases with dual-signature
>           requirement, full audit trail.
> Ref: `§23.9`, `§26.2`, `§39`, `§40`
> **Duration:** ~2 weeks

### 8.1 Ops Console Endpoints

- [ ] All ops endpoints require: `Authorization: Bearer <ops-token>` +
      `X-Ops-Signature` (Ed25519 sig of request body by registered ops device)
- [ ] `GET /v1/ops/manual-reviews` — list pending cases
- [ ] `GET /v1/ops/manual-review/{mr_id}` — full case detail
      (encrypted doc refs, attestations, audit chain — no plaintext)
- [ ] `POST /v1/ops/manual-review/{mr_id}/decision` — REQUIRES
      two ops signatures (`§23.9`):
      - Body: `{ decision, ops_signatures: [{user_id, sig}, {user_id, sig}] }`
      - Single signature → `ERR_DUAL_SIGNATURE_REQUIRED`
      - On approval: OpenBao-signed `release_record`, proceed to delivery
      - Audit entry: `manual_review_decision`
- [ ] ✍️ Write `docs/runbooks/incident-response.md` (ref `§39`)
- [ ] ✍️ Write `docs/runbooks/manual-review-ops.md` (ref `§26.2`)

### 8.2 Fraud & Anomaly Detection
> Ref: `§40`, `§39`

- [ ] Redis counters + Sentry alerts:
      - `server_decrypt_attempts_total > 0` → CRITICAL (must always be 0)
      - `aead_failure_rate > 5/min` → WARN
      - `claim_token_failure_rate > 10/min` → WARN + investigate
      - Multiple simultaneous claims → `conflict_pending` + Sentry alert
      - Ops access outside business hours → Sentry INFO

### Phase 8 Acceptance Criteria

- [ ] Single ops sig on manual decision → rejected
- [ ] Two ops sigs with one forged → rejected
- [ ] `server_decrypt_attempts_total` increment fires Sentry CRITICAL
- [ ] Manual review creates properly signed `release_record`

***

## Phase 9 — Security Hardening & Fuzz Testing
> **Goal:** System withstands adversarial testing across memory, crypto,
>           network, and dependency layers.
> Ref: `§39 Threat Model`, `§4.14`, `§4.16`, `§4.18`
> **Duration:** ~2 weeks

### 9.1 Memory Security
> Ref: `§4.16 Memory Hygiene`

- [ ] All cryptographic key material wrapped in
      `dryoc::protected::LockedBytes` / `dryoc::protected::LockedBox`
      (note: exact path is `dryoc::protected` — verify against pinned version)
- [ ] Zeroize all temp key buffers immediately after use (zeroize crate)
- [ ] `mlock()` on startup for crypto worker thread stack pages
- [ ] Compile-time lint: `deny(clippy::print_stdout)` in `crypto-core`
- [ ] Semgrep rule: assert no `tracing::info!` / `log::info!` contains
      key-adjacent variable names
- [ ] Disable core dumps in production Docker container
      (`ulimit -c 0` in entrypoint)

### 9.2 Security Test Suite
> Ref: `§4.14 Errors, Edge Cases & Rejection Rules`

- [ ] Replay attack: resend same AEAD request + `X-Seq` → `ERR_REPLAY_DETECTED`
- [ ] Clock skew: timestamp > 5 min old → `ERR_REPLAY_OR_SKEW`
- [ ] Signature forgery: tampered attestation → `ERR_SIGNATURE_INVALID`
- [ ] Wrong recipient: beneficiary A unwraps B's envelope →
      `ERR_ENVELOPE_RECIPIENT_MISMATCH`
      - AEAD tamper: flip one bit → `ERR_AEAD_INTEGRITY`
      - Deprecated version: `crypto_version = "aead-old-v0"` →
        `ERR_CRYPTO_VERSION_UNSUPPORTED`
      - Server decrypt invariant: simulate decrypt call path → CI fails

### 9.3 Fuzz Testing
> Ref: `§4.18 Implementation Checklist Crypto`

- [ ] `cargo-fuzz` targets in `crates/crypto-core/src/tests/fuzz/`:
      - `fuzz_envelope_parsing` — random bytes as ShareEnvelope JSON
      - `fuzz_jcs_canonicalization` — arbitrary JSON input to JCS
      - `fuzz_aead_decrypt` — corrupted nonce + ciphertext + AD combinations
- [ ] Run each fuzz target for minimum 1 hour on release branch in CI

### 9.4 Dependency Security

- [ ] `cargo audit` in CI — fail on any RUSTSEC advisory
- [ ] `cargo deny` — deny GPL/AGPL licenses
- [ ] `trivy` container scan — fail on HIGH/CRITICAL CVEs in Docker image
- [ ] All Docker base image digests pinned
      (`FROM rust:1.77@sha256:...` — never `FROM rust:latest`)
- [ ] Weekly `security-scan.yml` runs `semgrep` + `gitleaks` + `trivy`

### Phase 9 Acceptance Criteria

- [ ] All 7 security tests pass (replay, skew, forgery, recipient,
      AEAD tamper, wrong version, server decrypt)
- [ ] Fuzz targets run 1h with zero panics or crashes
- [ ] `cargo audit` clean
- [ ] `trivy` zero HIGH/CRITICAL CVEs
- [ ] Memory: zero uninitialized reads in crypto paths (valgrind clean)
- [ ] No `print!` / `println!` / `dbg!` in `crypto-core` (clippy lint)

---

## Phase 10 — Observability & Production Launch
> **Goal:** Full production observability, automated backups, load tested,
>           documented, go-live.
> Ref: `§40 Ops & Monitoring`
> **Duration:** ~2 weeks

### 10.1 Metrics & Alerting
> Ref: `§40.4`

- [ ] `metrics` crate + `metrics-exporter-prometheus`
      Expose `/metrics` on internal port only (never public)
- [ ] Key metrics:
      - `api_request_duration_seconds{route, status}` histogram
      - `api_errors_total{route, error_code}` counter
      - `server_decrypt_attempts_total` — MUST always be 0 in prod
      - `aead_failures_total` — alert if > 5/min
      - `heartbeat_worker_lag_seconds` — alert if > 3600s
      - `claim_processing_duration_seconds` histogram
      - `job_queue_depth{job_type}` gauge
      - `db_query_duration_seconds{query}` histogram
      - `nonce_reuse_detected_total` — alert if > 0
- [ ] Grafana Cloud (free tier: 14-day retention)
      Import dashboards for all above metrics
- [ ] Alert rules:
      - `server_decrypt_attempts_total > 0` → CRITICAL (PagerDuty/email)
      - `nonce_reuse_detected_total > 0` → CRITICAL
      - `aead_failures_total rate > 5/min` → WARN
      - `heartbeat_worker_lag_seconds > 3600` → WARN
      - `job_queue_depth > 500` → WARN
      - Nightly backup job failure → CRITICAL

### 10.2 Automated Backups
> Ref: `§26 Operational Playbooks`

- [ ] `infra/scripts/backup.sh` — nightly cron:
      `pg_dump` → `openssl enc -aes-256-gcm`
      (key fetched from OpenBao at runtime, never hardcoded)
      → upload to B2 `tl-db-backups-prod/{date}.sql.gz.enc`
- [ ] Retention: 30 daily backups, 12 monthly
- [ ] `infra/scripts/restore-test.sh` — weekly:
      restore backup to ephemeral Postgres container,
      verify row counts match, assert migrations apply clean
- [ ] ✍️ Write `docs/runbooks/backup-restore.md`

### 10.3 Load Testing

- [ ] `k6` load test scripts:
      - Auth: 100 concurrent OPAQUE registrations
      - Vault write: 500 req/s item creation
      - Heartbeat: 1000 concurrent heartbeats
- [ ] Target: p99 latency < 200ms (ref `§2.2 Non-Functional Requirements`)
- [ ] Identify and fix slow `sqlx` queries (add missing indexes per `§8`)

### 10.4 API Documentation

- [ ] Generate OpenAPI 3.1 spec from Axum routes (`utoipa` crate)
- [ ] Publish to `/v1/openapi.json` — authenticated (not public)
- [ ] Internal Swagger UI at `/v1/docs` — ops access only

### 10.5 Go-Live Checklist

- [ ] All CI/CD gates green on `main`
- [ ] All Phase 0–9 acceptance criteria passed
- [ ] External security review of crypto implementation (`§4.18`)
- [ ] GDPR privacy policy + ToS published
- [ ] Incident response runbook validated (`§39`, `§40.6`)
- [ ] OpenBao unseal keys stored in ≥2 separate secure locations
      (NOT on Hetzner — different physical locations)
- [ ] Supabase Pro plan activated (PITR backups, no connection limit surprises)
- [ ] Sentry, PostHog, Grafana dashboards reviewed
- [ ] Rate limits tuned from load test results
- [ ] All Brevo email templates verified end-to-end in staging
- [ ] B2 bucket versioning + retention policies confirmed
- [ ] `docs/runbooks/` — all 4 runbooks written and reviewed by team

### Phase 10 Acceptance Criteria

- [ ] p99 < 200ms under load test scenarios
- [ ] Nightly backup succeeds and restore test passes in CI
- [ ] All Grafana dashboards rendering live data
- [ ] Zero CRITICAL alerts firing on clean system
- [ ] OpenAPI spec documents all v1 endpoints
- [ ] Go-live checklist 100% checked

---

## Master Checklist — All Phases

> Use this as your single progress tracker. Every checkbox maps to a
> concrete deliverable. Nothing is checked until acceptance criteria pass.

### Phase 0 — Foundation
- [ ] 0.1 Monorepo + Cargo workspace init
- [ ] 0.2 Hetzner CX22 provisioned + hardened
- [ ] 0.3 Supabase project + 6 schemas + RLS deny-by-default
- [ ] 0.4 OpenBao Transit + Ed25519 key + UNSEAL.md written
- [ ] 0.5 OpenBao KV + all 13 secrets populated
- [ ] 0.6 Backblaze B2 — 3 buckets created + versioning
- [ ] 0.7 Redis 7 deployed (internal only)
- [ ] 0.8 GitHub Actions: ci.yml + security-scan.yml + deploy.yml
- [ ] 0.9 Base Axum: middleware stack + /health + /v1/server-capabilities
- [ ] 0.9 shared-types: PolicyStatus (8 states) + AppError + CryptoVersion
- [ ] Phase 0 acceptance criteria all green

### Phase 1 — Auth & Identity
- [ ] 1.1 Supabase Auth + Brevo SMTP + 3 email templates
- [ ] 1.2 OPAQUE register/login endpoints
- [ ] 1.2 CI invariant: no server decrypt path (test asserts)
- [ ] 1.3 App-layer AEAD transport + X-Seq replay protection
- [ ] 1.3 Idempotency key: X-Idempotency-Key cached in Redis
- [ ] 1.4 Device registration + Ed25519 device sig verification
- [ ] 1.5 TOTP enroll/verify + WebAuthn register/authenticate
- [ ] 1.5 Step-up challenges for 4 sensitive action types
- [ ] 1.6 person_id / user_id identity model + schemas
- [ ] 1.7 Logout + refresh + password reset
- [ ] ADR 003-opaque-over-srp.md written
- [ ] Phase 1 acceptance criteria all green

### Phase 2 — Vault
- [ ] 2.1 vault.items + vault.shares schema + RLS + index
- [ ] 2.2 Item CRUD (no decrypt path, CI enforced)
- [ ] 2.3 Share/pre-wrap + grant_sig Ed25519 verification
- [ ] 2.4 Crypto version migration path (login flag + migration endpoint)
- [ ] 2.4 Worker: weekly scan for deprecated crypto_version items
- [ ] 2.5 /v1/server-capabilities endpoint
- [ ] ADR 004-hybrid-kem-design.md written
- [ ] Phase 2 acceptance criteria all green

### Phase 3 — Policy & Heartbeat
- [ ] 3.1 inheritance.policies schema — all 8 status states
- [ ] 3.1 PostgreSQL trigger enforcing all valid transitions
- [ ] 3.1 inheritance.heartbeats schema
- [ ] 3.2 Policy create/update + pending_at / grace_deadline calculation
- [ ] 3.3 Heartbeat endpoint + device sig + revival logic
- [ ] 3.4 Invite system + HMAC claim tokens + atomic consumption
- [ ] ADR 005-supabase-schema-split.md written
- [ ] Phase 3 acceptance criteria all green

### Phase 4 — Workers & Notifications
- [ ] 4.1 apalis + Redis worker infra + DLQ → ops.failed_jobs
- [ ] 4.2 Heartbeat eval worker (all 4 cadences — §21)
- [ ] 4.3 Notification worker: 7 Brevo template IDs wired
- [ ] 4.3 notify.notification_log populated per send attempt
- [ ] 4.4 Daily audit anchor worker → OpenBao sign → B2 upload
- [ ] Phase 4 acceptance criteria all green

### Phase 5 — Claims, Files & Attestations
- [ ] 5.1 claims + claim_attachments + attestations + release_records schema
- [ ] 5.2 Claim initiation + Ed25519 sig verify + Type A window
- [ ] 5.3 B2 presign + HeadObject confirm + SHA256 verify
- [ ] 5.4 Attestation endpoint + optional Dilithium-2 + UNIQUE constraint
- [ ] 5.5 m-of-n evaluation worker (all preconditions — §23.7)
- [ ] Phase 5 acceptance criteria all green

### Phase 6 — Release & Delivery
- [ ] 6.1 Release record canonical JSON + OpenBao signing
- [ ] 6.1 conflict_hold_until set (default 48h)
- [ ] 6.2 Conflict detection → conflict_pending → manual_review
- [ ] 6.3 Envelope delivery worker + GET /v1/inheritance/envelopes
- [ ] 6.4 Evidence package endpoint
- [ ] runbooks/conflict-resolution.md written
- [ ] Phase 6 acceptance criteria all green

### Phase 7 — Audit & Compliance
- [ ] 7.1 audit.events: append-only RLS, prev_hash chain
- [ ] 7.1 GET audit chain + chain verify endpoint
- [ ] 7.2 GDPR export (encrypted) + erasure (crypto shredding)
- [ ] 7.2 ip_hash used everywhere — raw IP never stored
- [ ] Phase 7 acceptance criteria all green

### Phase 8 — Ops & Manual Review
- [ ] 8.1 Ops endpoints: list/view/decision (dual-sig enforced)
- [ ] 8.1 ERR_DUAL_SIGNATURE_REQUIRED enforced
- [ ] 8.2 Redis fraud counters + Sentry alert rules
- [ ] runbooks/incident-response.md written
- [ ] runbooks/manual-review-ops.md written
- [ ] Phase 8 acceptance criteria all green

### Phase 9 — Security Hardening
- [ ] 9.1 dryoc::protected wrappers + zeroize on all key material
- [ ] 9.1 Core dumps disabled in production
- [ ] 9.2 All 7 security test cases pass
- [ ] 9.3 Fuzz: 3 targets run 1h clean
- [ ] 9.4 cargo audit + cargo deny + trivy all clean
- [ ] Phase 9 acceptance criteria all green

### Phase 10 — Launch
- [ ] 10.1 Prometheus metrics + Grafana dashboards + all alert rules
- [ ] 10.2 Nightly backup + weekly restore-test passing
- [ ] 10.3 k6 load tests: p99 < 200ms
- [ ] 10.4 OpenAPI 3.1 spec generated
- [ ] 10.5 Go-live checklist 100% checked
- [ ] runbooks/backup-restore.md written
- [ ] Phase 10 acceptance criteria all green

---

## Rules Files Quick Reference

| File | When to read |
|------|-------------|
| `rules/security.rules.md` | Before every PR — non-negotiable |
| `rules/crypto.rules.md` | Before any crypto-core change |
| `rules/api.rules.md` | Before adding any endpoint |
| `rules/db.rules.md` | Before any migration or query |
| `rules/code.rules.md` | Before writing any business logic |
| `rules/git.rules.md` | Before branching or committing |
| `rules/infra.rules.md` | Before any infra/Docker/secret change |

---

## Invariants — Must Never Be Violated

These are enforced by CI tests, DB triggers, and code review:

1. **Server never decrypts user secrets** — zero decrypt functions in
   `crates/api` or `crates/worker`. CI test asserts. (`§5.11`)
2. **Audit entry on every state change** — every status transition,
   heartbeat, claim, attestation, release, manual decision writes an
   audit entry with `prev_hash` chaining. (`§4.8`)
3. **Dual operator approval for manual release decisions** — single
   signature rejected at handler level. (`§23.9`)
4. **No plaintext secrets in logs, traces, Sentry events** — Sentry
   `before_send` strips them; `deny(clippy::print_stdout)` in
   crypto-core; semgrep rule scans all crates.
5. **No fallback data, no embedded defaults, no silent degradation** —
   if a secret is missing at startup, the process exits. Never serve
   stale or embedded fallback responses.
6. **All signed JSON must be JCS-canonicalized before hashing** —
   non-canonical signatures rejected. (`§4.8`)
7. **`crypto_version` + `schema_version` required on every record** —
   unsupported version → hard reject, never silent coercion. (`§4.15`)
8. **PostgreSQL transition trigger is the single source of truth for
   policy state** — application code cannot bypass it.

