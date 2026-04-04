BEGIN;

CREATE TABLE IF NOT EXISTS audit.events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,
    payload_hash BYTEA NOT NULL,
    prev_hash BYTEA NULL,
    event_hash BYTEA NOT NULL,
    actor_id UUID NULL,
    ip_hash BYTEA NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_audit_events_policy_id ON audit.events(policy_id);

COMMIT;
