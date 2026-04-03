# Security Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: threat model, memory hygiene, audit, release control, signing, and incident handling.

## Absolute prohibitions
- Never store plaintext passwords, MK, IK, KEK, EMK, seed phrases, recovery shares, or decrypted user documents on the server.[file:1]
- Never implement server-side decryption for vault items or beneficiary envelopes; the design requires client-only decryption and zero-knowledge storage.[file:1]
- Never log secrets, ciphertext payloads, tokens, raw request bodies, or cryptographic material.
- Never expose Redis, PostgreSQL, or OpenBao publicly; internal network only.
- Never use fallback embedded data, fallback keys, fallback config, or silent degraded-security modes.
- Never bypass audit logging for any state-changing action related to auth, vault, policy, claims, attestation, release, or manual review.[file:1]
- Never allow a single operator to finalize manual release decisions; dual-operator approval is required.[file:1]

## Mandatory controls
- All sensitive HTTP bodies must use app-layer AEAD over TLS, with timestamp and monotonic sequence checks to detect replay or skew.[file:1]
- All signed JSON must be canonicalized with JCS before hashing or signature verification.[file:1]
- All secrets in memory must use protected wrappers plus zeroization on drop; long-lived sensitive buffers must use locked memory where practical.
- All release decisions and daily audit anchors must be signed by the server signing system, separate from user secret handling.[file:1]
- All mutating endpoints must require auth, authorization, validation, idempotency where relevant, and an audit entry.[file:1]
- Sentry must scrub all fields matching secret-like names before event submission.
- Core dumps must be disabled in production.

## Security review checklist
- No new decrypt path added to API or worker.
- No plaintext file handling on server.
- No secret-bearing logs.
- Replay, tamper, signature-failure, and nonce-reuse behavior tested.
- Audit entry added for every new state transition.
- Threat model updated if a new component or trust boundary is introduced.
