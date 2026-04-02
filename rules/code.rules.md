# Code Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: Rust crypto core, API contracts, testing, and implementation guidance.

## Language and structure
- Primary backend language: Rust.
- Use a monorepo Cargo workspace with separate crates for API, worker, crypto core, and shared types.
- Keep all cryptographic logic centralized in the crypto crate so future WASM and native consumers share the same core.[file:1]

## Style rules
- No `unwrap()` or `expect()` in request handling, workers, or core crypto logic.
- Strong typed errors only; map them to stable API error codes.
- Prefer explicit domain types over ad-hoc maps and loose JSON handling.
- Keep handlers thin; business logic belongs in services, and crypto belongs in the crypto crate.
- No dead code, TODO placeholders, or commented-out fallback code in merged branches.

## Logging and observability
- Use structured tracing only.
- No secrets, ciphertext, or user document contents in logs.
- Every request should carry a request ID through spans.
- Every worker job should emit start, retry, success, and failure telemetry.

## Testing rules
- Unit tests for all business rules.
- Integration tests for auth, vault, policy, claims, attestation, release, and audit flows.[file:1]
- Security tests for replay, signature forgery, tampered AEAD, version mismatch, and invalid state transitions.[file:1]
- End-to-end tests must cover register → create item → prewrap → missed heartbeat → claim → attestation → release flow, with signing stubbed as needed in CI.[file:1]

## Dependency rules
- Add few dependencies and prefer mature audited crates.
- Crypto and auth dependencies must be pinned carefully and reviewed before upgrades.
- CI must run formatting, linting, tests, audit, and dependency checks.
