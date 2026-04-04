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

CREATE TABLE IF NOT EXISTS notify.notification_log (
    notification_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NULL REFERENCES inheritance.policies(policy_id) ON DELETE SET NULL,
    recipient_email TEXT NOT NULL,
    template_id TEXT NOT NULL,
    status TEXT NOT NULL,
    error_message TEXT NULL,
    dedupe_key TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    sent_at TIMESTAMPTZ NULL
);

CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_log_dedupe ON notify.notification_log(dedupe_key);
CREATE INDEX IF NOT EXISTS idx_notification_log_policy ON notify.notification_log(policy_id);

COMMIT;
