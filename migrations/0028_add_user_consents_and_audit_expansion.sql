BEGIN;

-- 1. Add legal consent tracking columns to auth_ext.persons
ALTER TABLE auth_ext.persons 
ADD COLUMN IF NOT EXISTS terms_accepted_at TIMESTAMPTZ NULL,
ADD COLUMN IF NOT EXISTS terms_version TEXT NULL,
ADD COLUMN IF NOT EXISTS privacy_accepted_at TIMESTAMPTZ NULL,
ADD COLUMN IF NOT EXISTS privacy_version TEXT NULL,
ADD COLUMN IF NOT EXISTS consent_ip_hash BYTEA NULL,
ADD COLUMN IF NOT EXISTS consent_user_agent TEXT NULL;

-- 2. Expand audit.events to allow global system-level and user-level events (registration, consent, login, GDPR)
-- Allow policy_id to be NULL for non-policy security events
ALTER TABLE audit.events 
ALTER COLUMN policy_id DROP NOT NULL;

-- Fix the foreign key constraint so policy deletions do not trigger CASCADE DELETE on append-only audit logs
DO $$ 
DECLARE 
    fk_name text;
BEGIN
    SELECT tc.constraint_name INTO fk_name
    FROM information_schema.table_constraints AS tc 
    JOIN information_schema.key_column_usage AS kcu
      ON tc.constraint_name = kcu.constraint_name
      AND tc.table_schema = kcu.table_schema
    WHERE tc.constraint_type = 'FOREIGN KEY' 
      AND tc.table_schema = 'audit'
      AND tc.table_name = 'events'
      AND kcu.column_name = 'policy_id';

    IF fk_name IS NOT NULL THEN
        EXECUTE format('ALTER TABLE audit.events DROP CONSTRAINT %I', fk_name);
        EXECUTE 'ALTER TABLE audit.events ADD CONSTRAINT audit_events_policy_id_fkey FOREIGN KEY (policy_id) REFERENCES inheritance.policies(policy_id) ON DELETE SET NULL';
    END IF;
END $$;

-- 3. Add indexes for audit querying and performance
CREATE INDEX IF NOT EXISTS idx_audit_events_actor_id ON audit.events(actor_id);
CREATE INDEX IF NOT EXISTS idx_audit_events_event_type ON audit.events(event_type);
CREATE INDEX IF NOT EXISTS idx_audit_events_created_at ON audit.events(created_at);

COMMIT;
