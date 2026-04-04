BEGIN;

CREATE TABLE IF NOT EXISTS vault.items (
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

CREATE INDEX IF NOT EXISTS idx_vault_items_user_id ON vault.items(user_id);

CREATE TABLE IF NOT EXISTS vault.shares (
    share_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id UUID NOT NULL REFERENCES vault.items(item_id) ON DELETE CASCADE,
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

CREATE INDEX IF NOT EXISTS idx_vault_shares_owner_id ON vault.shares(owner_id);
CREATE INDEX IF NOT EXISTS idx_vault_shares_grantee_id ON vault.shares(grantee_id);
CREATE INDEX IF NOT EXISTS idx_vault_shares_item_id ON vault.shares(item_id);

COMMIT;
