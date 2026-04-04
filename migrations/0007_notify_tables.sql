BEGIN;

CREATE TABLE IF NOT EXISTS notify.invites (
    invite_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    email TEXT NOT NULL,
    role TEXT NOT NULL,
    claim_token_hmac BYTEA NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    used BOOLEAN NOT NULL DEFAULT false,
    used_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_invites_policy_id ON notify.invites(policy_id);
CREATE INDEX IF NOT EXISTS idx_invites_email ON notify.invites(email);

COMMIT;
