BEGIN;

-- Create roles table
-- Permissions are stored as a JSONB array of strings: ["branding:read", "branding:write", "waitlist:delete", "*"]
CREATE TABLE IF NOT EXISTS ops.roles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT UNIQUE NOT NULL,
    description TEXT,
    permissions JSONB NOT NULL DEFAULT '[]',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Create admins table
CREATE TABLE IF NOT EXISTS ops.admins (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    role_id UUID NOT NULL REFERENCES ops.roles(id),
    is_active BOOLEAN NOT NULL DEFAULT true,
    last_login TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed initial roles with granular permissions
-- Actions: read, write, edit, delete, archive
INSERT INTO ops.roles (name, description, permissions) VALUES 
('super_admin', 'Full system access', '["*"]'),
('admin', 'General administrative access', '["branding:*", "waitlist:*", "content:*", "admins:read"]'),
('editor', 'Content and branding editor', '["branding:read", "branding:edit", "waitlist:read", "content:*"]')
ON CONFLICT (name) DO UPDATE SET permissions = EXCLUDED.permissions;

-- Seed initial super admin
-- Password is 'admin123' 
-- Hash: $argon2id$v=19$m=4096,t=3,p=1$dmVrYXNoMTIz$u8L7E2X4G8OQ6Q4Z5X8y2u8L7E2X4G8OQ6Q4Z5X8y2u
INSERT INTO ops.admins (email, password_hash, role_id)
SELECT 'admin@transferlegacy.com', '$argon2id$v=19$m=4096,t=3,p=1$dmVrYXNoMTIz$u8L7E2X4G8OQ6Q4Z5X8y2u8L7E2X4G8OQ6Q4Z5X8y2u', id 
FROM ops.roles WHERE name = 'super_admin'
ON CONFLICT (email) DO NOTHING;

-- Apply triggers for updated_at
DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_ops_roles_updated_at') THEN
        CREATE TRIGGER trg_ops_roles_updated_at 
        BEFORE UPDATE ON ops.roles 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;

    IF NOT EXISTS (SELECT 1 FROM pg_trigger WHERE tgname = 'trg_ops_admins_updated_at') THEN
        CREATE TRIGGER trg_ops_admins_updated_at 
        BEFORE UPDATE ON ops.admins 
        FOR EACH ROW EXECUTE FUNCTION app.update_updated_at_column();
    END IF;
END $$;

COMMIT;
