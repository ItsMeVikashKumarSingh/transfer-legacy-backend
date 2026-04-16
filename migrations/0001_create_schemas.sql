BEGIN;

-- Essential schemas
CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS auth_ext;
CREATE SCHEMA IF NOT EXISTS core;
CREATE SCHEMA IF NOT EXISTS inheritance;
CREATE SCHEMA IF NOT EXISTS audit;
CREATE SCHEMA IF NOT EXISTS ops;
CREATE SCHEMA IF NOT EXISTS notify;

-- Minimal auth.users structure for local tests/dev if not using Supabase
CREATE TABLE IF NOT EXISTS auth.users (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE,
    created_at TIMESTAMPTZ DEFAULT now()
);

-- Extensions
CREATE EXTENSION IF NOT EXISTS pgcrypto;

-- pgaudit might not be available in local dev postgres images
DO $$ 
BEGIN 
    CREATE EXTENSION IF NOT EXISTS pgaudit;
EXCEPTION 
    WHEN OTHERS THEN 
        RAISE NOTICE 'Extension pgaudit could not be created, skipping...';
END $$;

COMMIT;
