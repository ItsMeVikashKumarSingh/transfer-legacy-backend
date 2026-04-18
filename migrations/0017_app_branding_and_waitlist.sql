BEGIN;

-- Create Schema
CREATE SCHEMA IF NOT EXISTS app;

-- CMS Table (JSON-based as per user preference)
CREATE TABLE IF NOT EXISTS app.content (
    slug TEXT PRIMARY KEY,
    body JSONB NOT NULL,
    metadata JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

-- Site Settings (Singleton)
CREATE TABLE IF NOT EXISTS app.settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    brand_name TEXT NOT NULL,
    logo_url TEXT NULL,
    support_email TEXT NULL,
    theme_config JSONB NULL,
    metadata JSONB NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Initial fallback/default entry for singleton
INSERT INTO app.settings (id, brand_name) 
VALUES (1, 'Transfer Legacy')
ON CONFLICT (id) DO NOTHING;

-- Waitlist Table
CREATE TABLE IF NOT EXISTS app.waitlist (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    name TEXT NULL,
    meta JSONB NULL, -- Capturing UTMs, browser, referrer etc.
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    deleted_at TIMESTAMPTZ NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false
);

-- Trigger Function for updated_at
CREATE OR REPLACE FUNCTION app.update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = now();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Apply triggers
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_app_settings_updated_at') THEN
        CREATE TRIGGER trg_app_settings_updated_at 
        BEFORE UPDATE ON app.settings 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_app_content_updated_at') THEN
        CREATE TRIGGER trg_app_content_updated_at 
        BEFORE UPDATE ON app.content 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_app_waitlist_updated_at') THEN
        CREATE TRIGGER trg_app_waitlist_updated_at 
        BEFORE UPDATE ON app.waitlist 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;
END $$;

COMMIT;
