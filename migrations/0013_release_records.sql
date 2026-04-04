BEGIN;

CREATE TABLE IF NOT EXISTS inheritance.release_records (
    release_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    claim_id UUID NOT NULL REFERENCES inheritance.claims(claim_id) ON DELETE CASCADE,
    payload_hash BYTEA NOT NULL,
    signature TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    crypto_version TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_release_records_policy_id ON inheritance.release_records(policy_id);

COMMIT;
