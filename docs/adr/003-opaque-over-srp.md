# ADR 003: OPAQUE Over SRP

## Status
Accepted

## Context
Password authentication must not expose password-equivalent material to the server. The system requires a secure PAKE that enables password-based login while keeping the server zero-knowledge of the password.

## Decision
Use OPAQUE (opaque-ke v4) with Ristretto255 + HKDF-SHA512 for registration and login. Server state for in-flight sessions is stored in Redis with a short TTL.

## Consequences
- Server never stores or sees plaintext passwords.
- Registration and login require multi-step client-server exchange.
- Redis becomes part of the auth flow for transient protocol state.
