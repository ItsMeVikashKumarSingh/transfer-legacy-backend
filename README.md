# Transfer Legacy

Secure, production-grade backend for a digital inheritance vault. The system
enforces zero-knowledge storage, heartbeat-based liveness, claims and
attestation workflows, and tamper-evident audit trails.

## Core Principles
- Server never decrypts user vault data
- All sensitive payloads use app-layer AEAD
- Every state-changing action is audit-logged
- Strict policy state machine enforced at the DB layer

## Repository Layout
- `crates/api` — Axum HTTP API service
- `crates/worker` — Background jobs and schedulers
- `crates/crypto-core` — Canonical crypto utilities (shared)
- `crates/shared-types` — Domain models and error types
- `migrations` — SQL migrations (schemas, triggers, policies)
- `docs` — ADRs and runbooks
- `rules` — Non-negotiable engineering rules

## Development Workflow
1. Read `project_detail.md` for canonical requirements.
2. Follow `DEVELOPMENT_PLAN.md` phases and acceptance criteria.
3. Review all files in `rules/` before making changes.

## Local Setup (Development)
- Configure `.env.local` with required variables.
- Run API and worker containers via Docker Compose (see `infra/`).

## Security & Compliance
- Audit events are append-only and chain-verified.
- All signing uses OpenBao Transit (no cloud KMS dependency).
- B2 is used for encrypted attachments and audit anchors.

## Status
Implementation is complete through Phase 7. Acceptance tests and infra
verification are still pending per the development plan.
