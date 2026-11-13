BEGIN;

-- Add avatar_url to auth_ext.persons
ALTER TABLE auth_ext.persons 
ADD COLUMN IF NOT EXISTS avatar_url TEXT NULL;

-- Create core.item_attachments table for Vault asset documents
CREATE TABLE IF NOT EXISTS core.item_attachments (
    attachment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    item_id UUID NOT NULL REFERENCES core.items(item_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    file_name TEXT NOT NULL,
    mime_type TEXT NOT NULL,
    size_bytes BIGINT NOT NULL,
    sha256 BYTEA NULL,
    object_key TEXT NOT NULL,
    is_encrypted BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false
);

CREATE INDEX IF NOT EXISTS idx_item_attachments_item_id ON core.item_attachments(item_id);
CREATE INDEX IF NOT EXISTS idx_item_attachments_user_id ON core.item_attachments(user_id);

COMMIT;
