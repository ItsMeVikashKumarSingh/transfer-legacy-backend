BEGIN;

DO $$ BEGIN
    CREATE TYPE inheritance.policy_type AS ENUM ('direct_transfer', 'm_of_n');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE inheritance.cadence AS ENUM ('1w', '15d', '1m', '3m');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE inheritance.policy_status AS ENUM (
        'active',
        'pending',
        'investigating',
        'release_ready',
        'conflict_pending',
        'manual_review',
        'released',
        'cancelled'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS inheritance.policies (
    policy_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    policy_type inheritance.policy_type NOT NULL,
    cadence inheritance.cadence NOT NULL,
    m_of_n JSONB NULL,
    beneficiaries JSONB NOT NULL,
    approvers JSONB NOT NULL,
    release_conditions JSONB NULL,
    status inheritance.policy_status NOT NULL DEFAULT 'active',
    last_heartbeat_at TIMESTAMPTZ NULL,
    pending_at TIMESTAMPTZ NULL,
    grace_deadline TIMESTAMPTZ NULL,
    conflict_hold_until TIMESTAMPTZ NULL,
    audit_head_hash BYTEA NULL,
    crypto_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_policies_owner_id ON inheritance.policies(owner_id);
CREATE INDEX IF NOT EXISTS idx_policies_status ON inheritance.policies(status);

CREATE TABLE IF NOT EXISTS inheritance.heartbeats (
    heartbeat_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES auth_ext.devices(device_id) ON DELETE CASCADE,
    device_sig BYTEA NOT NULL,
    ts TIMESTAMPTZ NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_heartbeats_policy_id ON inheritance.heartbeats(policy_id);

COMMIT;
