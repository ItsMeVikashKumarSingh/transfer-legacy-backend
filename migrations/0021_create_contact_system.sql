-- Migration: Create professional contact system
BEGIN;

-- Contact Configuration (Singleton)
CREATE TABLE IF NOT EXISTS app.contact_config (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    office_address TEXT NULL,
    map_embed_url TEXT NULL,
    emails JSONB NOT NULL DEFAULT '[]', -- List of { label: string, email: string }
    phones JSONB NOT NULL DEFAULT '[]', -- List of { label: string, number: string }
    social_links JSONB NOT NULL DEFAULT '{}', -- Key-value pairs for socials
    working_hours JSONB NOT NULL DEFAULT '[]', -- List of { days: string, hours: string }
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Initialize singleton
INSERT INTO app.contact_config (id) VALUES (1) ON CONFLICT (id) DO NOTHING;

-- Contact Messages
CREATE TABLE IF NOT EXISTS app.contact_messages (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    email TEXT NOT NULL,
    subject TEXT NULL,
    message TEXT NOT NULL,
    metadata JSONB NULL,
    is_read BOOLEAN NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Activity Logs Table (Aligning with temp backend and ops needs)
CREATE TABLE IF NOT EXISTS ops.activity_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id UUID NULL REFERENCES ops.admins(id),
    action TEXT NOT NULL,
    entity_type TEXT NULL,
    entity_id TEXT NULL,
    metadata JSONB NULL,
    ip_address TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Apply updated_at trigger to contact_config
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_app_contact_config_updated_at') THEN
        CREATE TRIGGER trg_app_contact_config_updated_at 
        BEFORE UPDATE ON app.contact_config 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;
END $$;

COMMIT;
