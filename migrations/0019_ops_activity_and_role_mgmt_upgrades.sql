BEGIN;

-- 1. Create Activity Logs Table
CREATE TABLE IF NOT EXISTS ops.activity_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    admin_id UUID REFERENCES ops.admins(id),
    action TEXT NOT NULL, -- e.g., 'login', 'update_branding', 'create_admin'
    entity_type TEXT NULL, -- e.g., 'admin', 'branding', 'role'
    entity_id TEXT NULL,
    metadata JSONB NULL, -- To store details like old_value and new_value
    ip_address TEXT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. Index for filtering logs
CREATE INDEX IF NOT EXISTS idx_ops_activity_logs_created_at ON ops.activity_logs(created_at DESC);
CREATE INDEX IF NOT EXISTS idx_ops_activity_logs_admin_id ON ops.activity_logs(admin_id);
CREATE INDEX IF NOT EXISTS idx_ops_activity_logs_action ON ops.activity_logs(action);

-- 3. Initial CMS content seeds if not already present
INSERT INTO app.content (slug, body) 
VALUES 
    ('hero', '{"title": "Secure Your Legacy", "subtitle": "Transfer your digital assets safely to your loved ones."}'),
    ('faqs', '{"items": [{"q": "How secure is it?", "a": "We use end-to-end encryption with your master password."}, {"q": "Is it free?", "a": "Basic inheritance is free, premium features require a subscription."}]}')
ON CONFLICT (slug) DO NOTHING;

COMMIT;
