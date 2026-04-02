# Database Rules — Transfer Legacy Backend

Reference sections in `project_detail.md`: architecture, ERD, identity model, heartbeat state, claims, attestations, audit, and release handling.

## Database choice and layout
- Use PostgreSQL on Supabase as the primary database.
- Use multiple schemas: `auth`, `vault`, `inheritance`, `audit`, `ops`, and `notify`.
- Keep relational integrity for users, persons, devices, items, shares, policies, claims, attestations, release records, and audit entries.
- Use JSONB only where flexibility is genuinely needed, not as a substitute for schema design.

## Data rules
- Separate `person` identity from `user account` identity to support email changes and re-onboarding while preserving long-term beneficiary/approver relationships.[file:1]
- Store ciphertext blobs and metadata only; no plaintext secrets or plaintext documents.[file:1]
- PII fields should be encrypted at the application layer before persistence.
- Every mutable business record must include created/updated timestamps.
- Every cryptographic record must persist version metadata.

## Constraints and integrity
- Enforce foreign keys and unique constraints.
- Enforce allowed status transitions at the DB layer as well as the application layer.
- Audit table must be append-only and chained by previous-hash references.[file:1]
- Release record creation, policy status updates, and audit writes must occur inside a single transaction when part of one logical action.[file:1]

## Query rules
- Use compile-time checked SQL where possible.
- No raw string-built SQL.
- No `SELECT *` in production code.
- Every high-cardinality lookup must have an index, especially by owner, policy status, pending timestamps, claim policy, and attestor.[file:1]

## Migrations
- Every migration must be reversible or explicitly documented as irreversible.
- No destructive production migration without a staged rollout plan.
- Schema changes affecting auth, crypto, release, or audit must include integration tests.
