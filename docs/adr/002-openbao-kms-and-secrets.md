# ADR 002: OpenBao for KMS + Secrets

## Status
Accepted

## Context
The system needs local signing for release records and audit anchors without cloud vendor lock-in. Secrets must be managed outside of Docker images and Git.

## Decision
Use OpenBao Transit for signing and OpenBao KV for runtime secret retrieval. The API will authenticate via AppRole with a narrowly scoped policy.

## Consequences
- Self-hosted signing with mlock and local-only access.
- Secrets are injected at runtime and never stored in the image.
- Requires explicit operational procedures (init, unseal, backup).
