# Transfer Legacy API Reference

This document covers all currently implemented API routes in `crates/api/src/router.rs`.

## Base URL

- Local: `http://127.0.0.1:<TL_PORT>`
- Versioned API prefix: `/v1`

## Response Formats

Most successful responses use:

```json
{
  "data": {},
  "request_id": "uuid"
}
```

Encrypted endpoints return:

```json
{
  "nonce": "base64url-no-pad",
  "ciphertext": "base64url-no-pad"
}
```

Error responses use:

```json
{
  "error": {
    "code": "ERR_*",
    "message": "Human readable message",
    "request_id": "uuid-or-unknown"
  }
}
```

## Headers

- `Authorization: Bearer <token>`: required only where explicitly consumed by handler/service.
- `x-idempotency-key`: required on endpoints that enforce idempotency.
- `x-request-id`: optional request header; always echoed in response envelope.
- `x-seq`, `x-timestamp`, `x-device-id`: required on all encrypted (`AEAD`) request bodies.
- `x-internal-token`: required for internal endpoints when `INTERNAL_API_TOKEN` is configured.

## AEAD Request Body

For endpoints marked as `AEAD Request`, send:

```json
{
  "nonce": "base64url-no-pad",
  "ciphertext": "base64url-no-pad"
}
```

Server decrypts this into the typed payload listed below.

---

## System Endpoints

### `GET /health`

- Auth: none
- Encryption: plain
- Response `HealthResponse`:
  - `status: "ok"`
  - `version: string` (build git sha or `unknown`)

### `GET /v1/server-capabilities`

- Auth: none
- Encryption: plain
- Response `ServerCapabilities`:
  - `crypto_versions: string[]`
  - `current_crypto_version: string`
  - `current_schema_version: number`
  - `aead: string`
  - `kdf: string`
  - `opaque_version: string`
  - `opaque_group: string`
  - `hybrid_kem: string`
  - `signatures: string[]`
  - `canonicalization: string`

### `GET /metrics`

- Auth: internal (`x-internal-token` when configured)
- Encryption: plain text metrics payload

### `GET /v1/openapi.json`

- Auth: internal (`x-internal-token` when configured)
- Encryption: plain JSON

### `GET /v1/docs`

- Auth: internal (`x-internal-token` when configured)
- Encryption: plain HTML

---

## Auth Endpoints

### `POST /v1/auth/register/init`

- Idempotency: required
- Request `RegisterInitRequest`:
  - `user_id: uuid`
  - `registration_request: string`
  - `credential_identifier?: string`
- Response `RegisterInitResponse`:
  - `session_id: uuid`
  - `registration_response: string`
  - `server_nonce: string`

### `PUT /v1/auth/register/finish`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Decrypted request `RegisterFinishRequest`:
  - `session_id: uuid`
  - `registration_upload: string`
  - `ed25519_pubkey: base64url`
  - `x25519_pubkey: base64url`
  - `kyber768_pubkey: base64url`
  - `emk_blob: base64url`
  - `argon2_params: object`
  - `enc_legal_name: base64url`
  - `enc_email: base64url`
- Decrypted success data `RegisterFinishResponse`:
  - `user_id: uuid`

### `POST /v1/auth/login/init`

- Idempotency: required
- Request `LoginInitRequest`:
  - `user_id: uuid`
  - `credential_request: string`
- Response `LoginInitResponse`:
  - `session_id: uuid`
  - `credential_response: string`
  - `server_nonce: string`

### `POST /v1/auth/login/finish`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Decrypted request `LoginFinishRequest`:
  - `session_id: uuid`
  - `credential_finalization: string`
- Decrypted success data `LoginFinishResponse`:
  - `user_id: uuid`
  - `session_token: string`
  - `emk_blob: base64url`
  - `argon2_params: object`
  - `ed25519_pubkey: base64url`
  - `x25519_pubkey: base64url`
  - `kyber768_pubkey: base64url`

### `POST /v1/auth/logout`

- Idempotency: required
- Optional auth header:
  - `Authorization: Bearer <access-token>`
- Response `LogoutResponse`:
  - `status: "ok"`

### `POST /v1/auth/refresh`

- Idempotency: required
- Request `RefreshRequest`:
  - `refresh_token: string`
- Response `RefreshResponse`:
  - `access_token: string`
  - `refresh_token: string`
  - `expires_in: number`

### `POST /v1/auth/password/reset/request`

- Idempotency: required
- Request `PasswordResetRequest`:
  - `email: string`
- Response `PasswordResetResponse`:
  - `status: "ok"`

### `POST /v1/auth/password/reset/confirm`

- Idempotency: required
- Request `PasswordResetConfirmRequest`:
  - `access_token: string`
  - `new_password: string`
- Response `PasswordResetResponse`:
  - `status: "ok"`

### `POST /v1/auth/mfa/totp/enroll`

- Idempotency: required
- Request `TotpEnrollRequest`:
  - `user_id: uuid`
- Response `TotpEnrollResponse`:
  - `otpauth_url: string`
  - `backup_codes: string[]`

### `POST /v1/auth/mfa/totp/verify`

- Idempotency: not required
- Request `TotpVerifyRequest`:
  - `user_id: uuid`
  - `code: string`
- Response `TotpVerifyResponse`:
  - `status: "ok"`

### `POST /v1/auth/mfa/webauthn/register/start`

- Idempotency: required
- Request `WebAuthnStartRequest`:
  - `user_id: uuid`
- Response `WebAuthnStartResponse`:
  - `challenge_id: uuid`
  - `challenge_b64: base64url`

### `POST /v1/auth/mfa/webauthn/register/finish`

- Idempotency: required
- Request `WebAuthnFinishRequest`:
  - `user_id: uuid`
  - `challenge_id: uuid`
  - `credential_id: string`
  - `public_key_b64: base64url`
  - `signature_b64: base64url`
  - `authenticator_data_b64: base64url`
  - `client_data_json_b64: base64url`
- Response `WebAuthnFinishResponse`:
  - `status: "ok"`

### `POST /v1/auth/mfa/webauthn/authenticate/start`

- Idempotency: required
- Request `WebAuthnStartRequest`:
  - `user_id: uuid`
- Response `WebAuthnStartResponse`:
  - `challenge_id: uuid`
  - `challenge_b64: base64url`

### `POST /v1/auth/mfa/webauthn/authenticate/finish`

- Idempotency: required
- Request `WebAuthnFinishRequest`:
  - `user_id: uuid`
  - `challenge_id: uuid`
  - `credential_id: string`
  - `signature_b64: base64url`
  - `authenticator_data_b64: base64url`
  - `client_data_json_b64: base64url`
- Response `WebAuthnFinishResponse`:
  - `status: "ok"`

### `POST /v1/auth/stepup/request`

- Idempotency: required
- Request `StepUpRequest`:
  - `user_id: uuid`
  - `action: string`
  - `challenge_type: string` (currently validated against `totp` in verification flow)
- Response `StepUpResponse`:
  - `challenge_id: uuid`
  - `expires_at: datetime`

### `POST /v1/auth/stepup/verify`

- Idempotency: required
- Request `StepUpVerifyRequest`:
  - `challenge_id: uuid`
  - `code: string`
- Response `StepUpVerifyResponse`:
  - `status: "ok"`

---

## Device Endpoints

### `POST /v1/devices/register`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Decrypted request `DeviceRegisterRequest`:
  - `device_id: uuid`
  - `user_id: uuid`
  - `ts: unix seconds`
  - `device_sig: base64url`
  - `ed25519_pubkey: base64url`
  - `device_meta?: object`
- Decrypted success data `DeviceRegisterResponse`:
  - `device_id: uuid`

### `POST /v1/devices/`

- Idempotency: not required
- Encryption: plain
- Request `DeviceListRequest`:
  - `user_id: uuid`
- Response `DeviceListResponse`:
  - `devices: DeviceListItem[]`
  - Item fields:
    - `device_id: uuid`
    - `ed25519_pubkey: base64url`
    - `device_meta?: object`
    - `created_at: datetime`
    - `last_seen_at?: datetime`

### `DELETE /v1/devices/:device_id`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Decrypted request `DeviceRevokeRequest`:
  - `user_id: uuid`
- Decrypted success data `DeviceRevokeResponse`:
  - `status: "ok"`

---

## Vault Endpoints

All vault endpoints use AEAD request + AEAD response.

### `POST /v1/vault/items`

- Idempotency: required
- Request `CreateItemRequest`:
  - `user_id: uuid`
  - `ciphertext: base64url`
  - `item_meta?: object`
  - `crypto_version: string`
- Response `CreateItemResponse`:
  - `item_id: uuid`

### `POST /v1/vault/items/list`

- Idempotency: not required
- Request `ListItemsRequest`:
  - `user_id: uuid`
- Response `ListItemsResponse`:
  - `items: ItemSummary[]`

### `POST /v1/vault/items/get`

- Idempotency: not required
- Request `GetItemRequest`:
  - `user_id: uuid`
  - `item_id: uuid`
- Response `GetItemResponse`:
  - `item_id: uuid`
  - `ciphertext: base64url`
  - `item_meta?: object`
  - `created_at: datetime`

### `POST /v1/vault/items/delete`

- Idempotency: required
- Request `DeleteItemRequest`:
  - `user_id: uuid`
  - `item_id: uuid`
- Response `DeleteItemResponse`:
  - `status: "ok"`

### `POST /v1/vault/shares`

- Idempotency: required
- Request `CreateShareRequest`:
  - `owner_id: uuid`
  - `item_id: uuid`
  - `grantee_id: uuid`
  - `envelope: object`
  - `grant_sig: base64url`
  - `crypto_version: string`
- Response `CreateShareResponse`:
  - `share_id: uuid`

### `POST /v1/vault/shares/list`

- Idempotency: not required
- Request `ListSharesRequest`:
  - `owner_id: uuid`
- Response `ListSharesResponse`:
  - `shares: ShareSummary[]`

### `POST /v1/vault/shares/revoke`

- Idempotency: required
- Request `RevokeShareRequest`:
  - `owner_id: uuid`
  - `share_id: uuid`
- Response `RevokeShareResponse`:
  - `status: "ok"`

### `POST /v1/vault/migrate`

- Idempotency: required
- Request `MigrateRequest`:
  - `user_id: uuid`
  - `from_version: string`
  - `to_version: string`
  - `item_ids: uuid[]`
- Response `MigrateResponse`:
  - `status: "ok"`

---

## Inheritance Endpoints

### `PUT /v1/inheritance/policy`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `PolicyUpsertRequest`:
  - `owner_id: uuid`
  - `policy_id?: uuid`
  - `policy_type: string`
  - `cadence: string` (`1w`, `15d`, `1m`, `3m`)
  - `m_of_n?: object` (required if `policy_type == "m_of_n"`, with valid `m`, `n`)
  - `beneficiaries: json`
  - `approvers: json`
  - `release_conditions?: json`
  - `stepup_challenge_id: uuid`
- Response `PolicyUpsertResponse`:
  - `policy_id: uuid`
  - `pending_at: datetime`
  - `grace_deadline: datetime`

### `POST /v1/inheritance/heartbeat`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `HeartbeatRequest`:
  - `policy_id: uuid`
  - `device_id: uuid`
  - `ts: unix seconds`
  - `device_sig: base64url`
- Response `HeartbeatResponse`:
  - `policy_id: uuid`
  - `pending_at: datetime`
  - `grace_deadline: datetime`
  - `status: string`

### `POST /v1/inheritance/policy/:policy_id/invite`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `InviteRequest`:
  - `email: string`
  - `role: "beneficiary" | "approver"`
  - `stepup_challenge_id: uuid`
- Response `InviteResponse`:
  - `invite_id: uuid`
  - `expires_at: datetime`

### `POST /v1/inheritance/claim-token/consume`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `ClaimTokenConsumeRequest`:
  - `invite_id: uuid`
  - `claim_token: base64url`
  - `person_id: uuid`
- Response `ClaimTokenConsumeResponse`:
  - `status: "ok"`

### `GET /v1/inheritance/envelopes`

- Idempotency: not required
- Query params:
  - `claim_id: uuid`
  - `claimant_person_id: uuid`
- Encryption: AEAD response
- Response `EnvelopesResponse`:
  - `policy_id: uuid`
  - `claim_id: uuid`
  - `items: EnvelopeItem[]`
  - Item fields:
    - `share_id: uuid`
    - `item_id: uuid`
    - `envelope_b64: base64url`
    - `grant_sig_b64: base64url`

### `POST /v1/inheritance/evidence-package`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `EvidencePackageRequest`:
  - `policy_id: uuid`
  - `claim_id: uuid`
- Response `EvidencePackageResponse`:
  - `evidence: json`
  - `signature: string` (OpenBao transit signature)

---

## Claims Endpoints

All claims endpoints use AEAD request + AEAD response and require idempotency.

### `POST /v1/claims/initiate`

- Request `ClaimInitiateRequest`:
  - `policy_id: uuid`
  - `claimant_person_id: uuid`
  - `claim_type: "type_a" | "type_b"`
- Response `ClaimInitiateResponse`:
  - `claim_id: uuid`
  - `confirmation_deadline?: datetime`

### `POST /v1/claims/confirm`

- Request `ClaimConfirmRequest`:
  - `claim_id: uuid`
  - `claimant_person_id: uuid`
- Response `ClaimConfirmResponse`:
  - `status: "ok"`

### `POST /v1/claims/attachments/presign`

- Request `PresignAttachmentRequest`:
  - `claim_id: uuid`
  - `content_type: string`
- Response `PresignAttachmentResponse`:
  - `attachment_id: uuid`
  - `upload_url: string`
  - `object_key: string`

### `POST /v1/claims/attachments/confirm`

- Request `ConfirmAttachmentRequest`:
  - `attachment_id: uuid`
  - `sha256_b64: base64url`
  - `size_bytes: number`
  - `mime_type: string`
- Response `ConfirmAttachmentResponse`:
  - `status: "ok"`

### `POST /v1/claims/attestations`

- Request `AttestationRequest`:
  - `policy_id: uuid`
  - `claim_id: uuid`
  - `approver_person_id: uuid`
  - `statement: json`
  - `signature_b64: base64url`
  - `public_key_b64: base64url`
  - `signature_type: "ed25519"`
- Response `AttestationResponse`:
  - `attestation_id: uuid`

### `POST /v1/claims/release-records`

- Request `ReleaseRecordRequest`:
  - `policy_id: uuid`
  - `claim_id: uuid`
  - `payload: json`
  - `schema_version: number`
  - `crypto_version: string`
- Response `ReleaseRecordResponse`:
  - `release_id: uuid`
  - `signature: string` (OpenBao transit signature)

---

## Audit Endpoint

### `GET /v1/audit/chain`

- Idempotency: not required
- Query:
  - `policy_id: uuid`
- Encryption: AEAD response
- Response `AuditChainResponse`:
  - `policy_id: uuid`
  - `valid: boolean`
  - `invalid_at?: number` (zero-based index where hash chain check fails)
  - `events: AuditChainEvent[]`

---

## GDPR Endpoints

Both GDPR endpoints use AEAD request + AEAD response and require idempotency.

### `POST /v1/gdpr/export`

- Request `GdprExportRequest`:
  - `user_id: uuid`
  - `person_id: uuid`
  - `export_key_b64: base64url` (must decode to 32 bytes)
- Response `GdprExportResponse`:
  - `nonce_b64: base64url`
  - `ciphertext_b64: base64url`

### `POST /v1/gdpr/erase`

- Request `GdprEraseRequest`:
  - `user_id: uuid`
  - `person_id: uuid`
- Response `GdprEraseResponse`:
  - `status: "ok"`

---

## Ops Endpoints

### `GET /v1/ops/reviews`

- Idempotency: not required
- Query:
  - `status?: string`
- Encryption: AEAD response
- Response: `ReviewSummary[]`

### `GET /v1/ops/reviews/:review_id`

- Idempotency: not required
- Encryption: AEAD response
- Response `ReviewDetail`:
  - `review_id: uuid`
  - `policy_id: uuid`
  - `conflict_id?: uuid`
  - `status: string`
  - `notes?: json`
  - `created_at: datetime`
  - `resolved_at?: datetime`

### `POST /v1/ops/reviews/:review_id/decision`

- Idempotency: required
- Encryption: AEAD request + AEAD response
- Request `ReviewDecisionRequest`:
  - `decision: "released" | "cancelled"`
  - `notes: json`
  - `operator_a_id: uuid`
  - `operator_a_public_key_b64: base64url`
  - `operator_a_signature_b64: base64url`
  - `operator_b_id: uuid`
  - `operator_b_public_key_b64: base64url`
  - `operator_b_signature_b64: base64url`
- Response `ReviewDecisionResponse`:
  - `status: "ok"`
  - `review_id: uuid`
  - `policy_id: uuid`

---

## Error Codes

Current application error codes:

- `ERR_BAD_REQUEST`
- `ERR_UNAUTHORIZED`
- `ERR_FORBIDDEN`
- `ERR_NOT_FOUND`
- `ERR_CONFLICT`
- `ERR_RATE_LIMITED`
- `ERR_INTERNAL`
- `ERR_AEAD_INTEGRITY`
- `ERR_REPLAY_DETECTED`
- `ERR_REPLAY_OR_SKEW`
- `ERR_SIGNATURE_INVALID`
- `ERR_ENVELOPE_RECIPIENT_MISMATCH`
- `ERR_CRYPTO_VERSION_UNSUPPORTED`
- `ERR_DUAL_SIGNATURE_REQUIRED`

## Postman Assets

- Collection: `postman/Transfer-Legacy.postman_collection.json`
- Environment: `postman/Transfer-Legacy.postman_environment.json`
