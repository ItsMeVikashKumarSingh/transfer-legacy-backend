---
title: "ADR 005: Supabase Schema Split"
date: "2026-04-04"
status: "accepted"
---

## Context
The platform stores distinct categories of data with different access patterns,
retention needs, and security requirements. Mixing these concerns in a single
schema increases the risk of overly broad permissions, accidental cross-domain
joins, and weaker audit boundaries.

## Decision
Split the PostgreSQL database into dedicated schemas and align each schema with
one domain and its security posture:

- `auth_ext`: identity, devices, MFA, step-up challenges, OPAQUE records
- `vault`: encrypted user items and shares
- `inheritance`: policies, heartbeats, claims, attestations, release records
- `audit`: append-only event chain for sensitive actions
- `ops`: operational records (conflicts, manual reviews, failed jobs)
- `notify`: invite tracking and notification logs

All schemas use RLS with deny-by-default. Service-role access is required for
mutations, while read access is scoped per domain.

## Consequences
- Clearer isolation between domains and easier RLS review.
- Lower blast radius for mistakes in grants or policy changes.
- Simpler migration and retention strategies per domain.
- Slightly more verbose SQL due to schema-qualified table names.
