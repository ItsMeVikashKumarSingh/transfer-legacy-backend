-- Migration: Grant necessary permissions for 'app' and 'ops' schemas to Supabase roles
BEGIN;

-- 1. Grant usage on schemas to allow the API to see them
GRANT USAGE ON SCHEMA app TO anon, authenticated, service_role;
GRANT USAGE ON SCHEMA ops TO anon, authenticated, service_role;

-- 2. Grant access to all existing tables in these schemas
GRANT ALL ON ALL TABLES IN SCHEMA app TO anon, authenticated, service_role;
GRANT ALL ON ALL TABLES IN SCHEMA ops TO anon, authenticated, service_role;

-- 3. Ensure any tables created in the future also get these permissions automatically
ALTER DEFAULT PRIVILEGES IN SCHEMA app GRANT ALL ON TABLES TO anon, authenticated, service_role;
ALTER DEFAULT PRIVILEGES IN SCHEMA ops GRANT ALL ON TABLES TO anon, authenticated, service_role;

-- 4. Reload the PostgREST cache so the changes take effect immediately
NOTIFY pgrst, 'reload schema';

COMMIT;
