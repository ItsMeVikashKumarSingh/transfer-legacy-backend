# project_detail.md

## 1. Project

**Name:** Transfer Legacy  
**Type:** Backend-only secure inheritance microservice  
**Goal:** Build a production-grade, zero-knowledge-oriented backend API for digital inheritance workflows: encrypted vault item storage, beneficiary share wrapping, heartbeat-based liveness, claims, attestations, controlled release, and tamper-evident audit trails.

## 2. Core Principles

- Security is the highest priority.
- No fallback data, embedded defaults, silent degradation, or insecure compatibility modes.
- The server must never decrypt user vault plaintext.
- Sensitive flows must be cryptographically verifiable, auditable, and replay-resistant.
- Code must be production-grade, maintainable, testable, cost-effective, and portable.
- Backend APIs only for now; frontend is out of scope.
- Documentation, runbooks, ADRs, and operational procedures are created during the relevant development phases, not as empty placeholders upfront.

## 3. Final Stack Decisions

### 3.1 Language and Runtime
- **Rust** (stable, pinned version)
- **Axum** for HTTP APIs
- **Tokio** for async runtime
- **sqlx** for compile-time checked SQL

### 3.2 Database
- **PostgreSQL via Supabase**
- Supabase is used primarily as a managed PostgreSQL platform
- Database design uses **multiple schemas**
- RLS is enabled where appropriate, with deny-by-default posture

### 3.3 Cache / Queue
- **Redis**
- Used for rate limiting, temporary auth state, replay protection, idempotency, and worker queue support

### 3.4 Crypto
- **OpenBao** for:
  - Transit signing / verification operations (KMS-like usage)
  - KV secret storage (secret management)
- **No separate secrets manager** because OpenBao already covers KMS + secrets
- **Argon2id** for password-derived key material
- **OPAQUE** for password-authenticated key exchange / password auth flow
- **XChaCha20-Poly1305** for AEAD payloads
- **Ed25519** for signatures
- **X25519 + Kyber/ML-KEM hybrid envelope approach** for share wrapping, where applicable
- **JCS canonicalization** before hashing/signing any canonical JSON payloads

### 3.5 Storage
- **Backblaze B2** instead of Cloudflare R2
- Used for encrypted claim attachments, evidence files, audit anchors, and backups

### 3.6 Hosting
- **Production:** Hetzner CX22
- **Staging:** same Hetzner box initially via Docker profiles / isolated services and config
- Separate staging VPS can be added later if needed
- railway can be used for testing purpose

### 3.7 Observability
- **Sentry** for error tracking with strict redaction
- **PostHog** for backend analytics / product telemetry
- Metrics suitable for Prometheus/Grafana-compatible scraping

### 3.8 Email
- **Brevo** initially
- Future migration path to **AWS SES**

### 3.9 Deployment Model
- Dockerized services
- Must remain portable enough for future migration to other VPS/cloud environments
- No code that assumes a single-host forever architecture

## 4. High-Level Architecture

The system is built as a backend microservice-oriented Rust workspace with a small number of focused services/crates:

- `api` — HTTP API service
- `worker` — background jobs, timers, scheduled evaluations
- `crypto-core` — cryptographic helpers and version-gated primitives
- `shared-types` — domain types, schema versions, shared errors

Primary external dependencies:

- Supabase PostgreSQL
- Redis
- OpenBao
- Backblaze B2
- Brevo
- Sentry
- PostHog

## 5. Repository Structure

```text
transfer-legacy/
├── crates/
│   ├── api/
│   ├── worker/
│   ├── crypto-core/
│   └── shared-types/
├── migrations/
├── infra/
├── rules/
├── docs/
│   ├── runbooks/
│   └── adr/
├── output/
├── .github/workflows/
├── Cargo.toml
└── README.md
```

## 6. Documentation Timing

These are intentionally written during development when behavior is real and finalized:

- `infra/openbao/UNSEAL.md`
- `docs/runbooks/incident-response.md`
- `docs/runbooks/conflict-resolution.md`
- `docs/runbooks/backup-restore.md`
- ADR files under `docs/adr/`

They are not created upfront as empty placeholders.

## 7. Domain Modules

### 7.1 Auth
- OPAQUE registration and login
- Session handling
- Device registration and device signature verification
- MFA / step-up authentication
- Password reset and refresh flows

### 7.2 Vault
- Encrypted item storage
- Metadata-only server handling
- Pre-wrapped share/envelope storage for beneficiaries
- No plaintext processing on server

### 7.3 Inheritance
- Policy creation/update
- Heartbeat submission and liveness evaluation
- Beneficiary / approver invites
- Claim initiation
- Attestations
- Release decisioning and envelope delivery readiness

### 7.4 Files
- Presigned upload workflow to Backblaze B2
- Confirmed attachment metadata storage
- No file proxying through API where avoidable

### 7.5 Audit
- Append-only chainable audit records
- Release evidence packaging
- Daily audit anchor generation and storage

### 7.6 Ops
- Manual review support
- Conflict handling
- Failed job inspection
- Dual-approval flows where required

## 8. Database Design

### 8.1 PostgreSQL Schemas
Use multiple PostgreSQL schemas:

- `auth_ext`
- `vault`
- `inheritance`
- `audit`
- `ops`
- `notify`

### 8.2 Standard Audit Columns
Every major table should include standard lifecycle and auditing columns unless a domain-specific reason prevents it:

```sql
created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
created_by   UUID NULL,
updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
updated_by   UUID NULL,
deleted_at   TIMESTAMPTZ NULL,
deleted_by   UUID NULL,
is_deleted   BOOLEAN NOT NULL DEFAULT false,
version      INTEGER NOT NULL DEFAULT 1
```

Additional notes:
- `updated_at` should be maintained automatically via trigger.
- `version` is used for optimistic concurrency control where relevant.
- Soft delete is preferred unless compliance or integrity rules require hard delete.
- Audit/event tables may use append-only semantics instead of update/delete columns where appropriate.

### 8.3 Core Tables
Representative table groups:

- `auth_ext.persons`
- `auth_ext.person_user_links`
- `auth_ext.opaque_records`
- `auth_ext.devices`
- `auth_ext.mfa_factors`
- `vault.items`
- `vault.shares`
- `inheritance.policies`
- `inheritance.heartbeats`
- `inheritance.claims`
- `inheritance.claim_attachments`
- `inheritance.attestations`
- `inheritance.release_records`
- `audit.events`
- `ops.manual_reviews`
- `ops.conflict_records`
- `ops.failed_jobs`
- `notify.invites`
- `notify.notification_log`

## 9. Security Requirements

### 9.1 Non-Negotiable Rules
- No server-side vault plaintext decryption.
- No secrets in logs, traces, analytics, or error payloads.
- No insecure fallback behavior.
- No embedded mock data in production paths.
- No plaintext secret storage in database or object storage.
- No silent crypto downgrades.
- No accepting unsupported `crypto_version` or `schema_version`.

### 9.2 Memory-Safety Direction
Rust is chosen specifically for strong memory safety guarantees. In crypto-sensitive paths:

- Minimize secret lifetimes in memory
- Use zeroization for sensitive buffers where applicable
- Avoid copying secret material unnecessarily
- Prefer protected-memory patterns where supported by chosen libraries
- Disable core dumps in production

### 9.3 Transport Security
- TLS at ingress
- Additional app-layer AEAD protection for sensitive request/response flows where defined
- Replay protection via sequence or nonce tracking
- Timestamp skew validation where required
- Idempotency keys for mutating routes

### 9.4 Signing and Canonicalization
All signed JSON payloads must:
1. be normalized via JCS canonicalization,
2. be hashed,
3. then signed / verified.

### 9.5 KMS / Secret Management Final Decision
**OpenBao is the single solution for both concerns:**

- **Transit engine** acts as KMS/signing service
- **KV engine** stores application secrets

This avoids paying for multiple overlapping platforms early.

## 10. Storage Requirements

### 10.1 Backblaze B2 Usage
Backblaze B2 stores:
- encrypted claim attachments,
- supporting evidence files,
- audit anchors,
- encrypted database backups.

### 10.2 File Rules
- Store encrypted content or encrypted references only
- Prefer presigned upload/download flows
- Verify hashes on confirmation where relevant
- Maintain metadata and evidence linkage in PostgreSQL

## 11. Environment Strategy

### 11.1 Production
- Hetzner CX22
- Dockerized runtime
- Caddy or equivalent reverse proxy
- OpenBao, Redis, API, worker on same host initially if resource usage permits
- Supabase remains managed externally
- Backblaze B2 remains managed externally

### 11.2 Staging
- Initially on the same Hetzner machine using isolated Docker profiles, separate env, separate secrets, separate database namespaces or dedicated staging database
- Keep staging logically isolated from production
- Do not rely on Fly.io free plan as a core environment strategy

### 11.3 Local Development
Use Docker Compose for:
- API
- worker
- Redis
- OpenBao
- optional local supporting services

## 12. Operational Services

### 12.1 Error Tracking
- Sentry with aggressive scrubbing/redaction
- Never send decrypted user data or secret-like payloads

### 12.2 Analytics
- PostHog server-side events only
- No sensitive content in analytics payloads

### 12.3 Notifications
- Brevo for transactional email
- Template-driven email flows
- Migration path to AWS SES kept in design

## 13. Background Jobs

Worker responsibilities include:
- heartbeat evaluation,
- reminder notifications,
- claim/release checks,
- conflict escalation,
- audit anchor generation,
- backup verification tasks,
- dead-letter handling.

All jobs must be:
- idempotent,
- retry-safe,
- observable,
- failure-reporting.

## 14. API Standards

- REST JSON APIs
- Versioned under `/v1`
- RFC 7807-style structured errors or equivalent consistent contract
- Strict request validation
- No ambiguous partial-success semantics
- Sensitive endpoints require auth, step-up, and/or signature verification as appropriate

## 15. Policy State Model

The inheritance policy model uses exactly these states:

- `active`
- `pending`
- `investigating`
- `release_ready`
- `conflict_pending`
- `manual_review`
- `released`
- `cancelled`

State transitions should be enforced at the database layer where possible.

## 16. Audit Model

The system maintains tamper-evident audit records for:
- auth events,
- device changes,
- policy changes,
- heartbeats,
- claims,
- attestations,
- release record creation,
- manual review decisions,
- evidence generation.

Design goals:
- append-only behavior,
- chainable hashes,
- daily signed anchor snapshots,
- evidence package generation for important release actions.

## 17. Rules Files

The repository contains and must enforce separate rules documents:

- `rules/security.rules.md`
- `rules/crypto.rules.md`
- `rules/api.rules.md`
- `rules/db.rules.md`
- `rules/code.rules.md`
- `rules/git.rules.md`
- `rules/infra.rules.md`

These are part of the implementation contract and should be referenced during development and review.

## 18. Development Planning

The phased implementation plan is maintained separately in:

- `DEVELOPMENT_PLAN.md`

That plan is the execution roadmap. This `project_detail.md` is the canonical product/architecture specification reference.

## 19. Cost-Conscious Strategy

To minimize recurring cost while staying production-grade:

- Use Hetzner CX22 for first production host
- Use same host for initial staging with strict isolation
- Use Supabase-managed Postgres instead of self-hosting Postgres
- Use OpenBao instead of separate KMS + secret manager products
- Use Backblaze B2 for low-cost object storage
- Use Brevo initially and migrate to SES later only when scale justifies it

## 20. Out of Scope for Now

- Frontend / client applications
- Public admin dashboard UI
- Mobile app implementation
- Multi-region deployment
- Fully separate staging VPS from day one
- Enterprise compliance certification execution work

## 21. Final Notes

- This project is backend-first and security-first.
- No assumptions should override explicit decisions captured here.
- If architecture decisions change later, record them in ADRs and update this file.
- Implementation must stay aligned with `DEVELOPMENT_PLAN.md` and the `rules/` directory.
