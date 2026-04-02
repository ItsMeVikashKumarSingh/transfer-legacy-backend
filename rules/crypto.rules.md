# Cryptography Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: primitive matrix, key hierarchy, OPAQUE, app-layer AEAD, item encryption, hybrid KEM, signatures, errors, rotation, and memory hygiene.

## Approved primitives only
- Symmetric encryption: XChaCha20-Poly1305.[file:1]
- Password KDF: Argon2id.[file:1]
- Authentication PAKE: OPAQUE.[file:1]
- Classical key agreement: X25519.[file:1]
- Post-quantum KEM: Kyber-768 / ML-KEM-768.[file:1]
- Signatures: Ed25519 required, Dilithium-2 optional for PQ signature support.[file:1]
- Hashing: SHA-256.[file:1]
- Canonicalization: JCS RFC 8785.[file:1]
- Hybrid beneficiary wrapping: X25519 + Kyber-768 combined through HKDF-SHA256.[file:1]

## Forbidden primitives and patterns
- No AES-CBC, ECB, unauthenticated encryption, bcrypt, PBKDF2, SHA-1, MD5, or custom cryptography.
- No deterministic or reused nonces.
- No signing of non-canonical JSON.
- No direct use of only X25519 or only Kyber for beneficiary envelope wrapping; hybrid mode only.[file:1]
- No server generation or storage of user MK or IK.[file:1]

## Implementation rules
- Every encrypted record must include `crypto_version` and `schema_version` and be rejected if unsupported.[file:1]
- Every item key is random per item and generated client-side only.[file:1]
- Every master key is random per user and generated client-side only.[file:1]
- Every AEAD failure must return a generic integrity error without revealing whether key, nonce, or payload was wrong.[file:1]
- Every signature verification must use the signer’s registered public key, not a public key supplied inline by an untrusted request.[file:1]
- Every change to cryptographic parameters requires compatibility tests and migration rules.[file:1]

## Memory rules
- Use protected memory for active secrets where feasible.
- Zeroize all temporary key buffers immediately after use.
- Avoid secret copies across async boundaries.
- Keep secrets out of panics, debug output, and tracing fields.
