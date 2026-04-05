BEGIN;

CREATE TABLE IF NOT EXISTS ops.conflict_records (
    conflict_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    reason TEXT NOT NULL,
    details JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_conflicts_policy_id ON ops.conflict_records(policy_id);

DO $$ BEGIN
    CREATE TYPE ops.manual_review_status AS ENUM ('open', 'resolved');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS ops.manual_reviews (
    review_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    conflict_id UUID NULL REFERENCES ops.conflict_records(conflict_id) ON DELETE SET NULL,
    status ops.manual_review_status NOT NULL DEFAULT 'open',
    notes JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    resolved_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_manual_reviews_policy_id ON ops.manual_reviews(policy_id);

COMMIT;
