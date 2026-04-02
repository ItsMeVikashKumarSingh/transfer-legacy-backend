# API Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: endpoint design, exact payloads, heartbeat, claims, attestations, releases, audit, and errors.

## General conventions
- Base path must be `/v1`.
- Use lowercase, hyphenated resource naming.
- Public endpoints must be minimal; all sensitive endpoints require AEAD-wrapped bodies or AEAD-wrapped responses where specified.[file:1]
- All requests must carry request ID, timestamp, and sequence metadata for replay protection on sensitive flows.[file:1]

## Response format
Use a single structured error envelope everywhere:

```json
{
  "error": {
    "code": "ERR_AEAD_INTEGRITY",
    "message": "Request integrity check failed.",
    "request_id": "uuid"
  }
}
```

Success format:

```json
{
  "data": {},
  "request_id": "uuid"
}
```

## Error code rules
- Keep MD-defined codes authoritative: integrity, auth-flow, replay/skew, replay-detected, invalid-signature, recipient-mismatch, unsupported-crypto-version, and standard auth/forbidden/conflict/internal families.[file:1]
- Never expose stack traces, SQL errors, internal object names, or security details in client responses.
- Use the same external auth failure shape for user enumeration resistance.

## Endpoint rules
- Auth flow must support full registration/login lifecycle, email verification, reset, MFA enrollment, session handling, and device registration.
- Vault endpoints store ciphertext and metadata only; plaintext never enters server business logic.[file:1]
- Policy endpoints must enforce the state machine strictly: active, pending, investigating, release_ready, conflict_pending, manual_review, released, cancelled.[file:1]
- Release endpoints must never deliver envelopes before conflict hold expiry unless explicitly resolved through the required process.[file:1]
- File upload endpoints must use presigned URLs and require client-side encryption before upload.[file:1]

## Rate limiting and idempotency
- Auth init/finish endpoints must be heavily rate limited.
- Claim, invite, file confirm, attestation, and release-trigger paths must require idempotency keys.
- Rate limit state belongs in Redis.
