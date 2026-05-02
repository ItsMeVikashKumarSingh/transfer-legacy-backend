-- Migration: Add waitlist_enabled and support fields to app.settings
BEGIN;

ALTER TABLE app.settings 
ADD COLUMN IF NOT EXISTS waitlist_enabled BOOLEAN NOT NULL DEFAULT true,
ADD COLUMN IF NOT EXISTS support_phone TEXT NULL,
ADD COLUMN IF NOT EXISTS support_address TEXT NULL;

-- Also ensure app.waitlist has 'name' and 'updated_at' if missing (it should have them from 0017)
-- But let's be safe.
ALTER TABLE app.waitlist
ADD COLUMN IF NOT EXISTS name TEXT NULL,
ADD COLUMN IF NOT EXISTS updated_at TIMESTAMPTZ NOT NULL DEFAULT now();

COMMIT;
