BEGIN;

CREATE TABLE IF NOT EXISTS core.items (
    item_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    ciphertext BYTEA NOT NULL,
    item_meta JSONB NULL,
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

CREATE INDEX IF NOT EXISTS idx_core_items_user_id ON core.items(user_id);

CREATE TABLE IF NOT EXISTS core.shares (
    share_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id UUID NOT NULL REFERENCES core.items(item_id) ON DELETE CASCADE,
    owner_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    grantee_id UUID NOT NULL,
    envelope BYTEA NOT NULL,
    grant_sig BYTEA NOT NULL,
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

CREATE INDEX IF NOT EXISTS idx_core_shares_owner_id ON core.shares(owner_id);
CREATE INDEX IF NOT EXISTS idx_core_shares_grantee_id ON core.shares(grantee_id);
CREATE INDEX IF NOT EXISTS idx_core_shares_item_id ON core.shares(item_id);

COMMIT;
