BEGIN;

-- 1. Create ops.user_metadata to store admin status and plan without modifying auth.users
CREATE TABLE IF NOT EXISTS ops.user_metadata (
    user_id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active',
    plan_tier TEXT NOT NULL DEFAULT 'Personal',
    last_active_at TIMESTAMPTZ NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 2. Create ops.plans table
CREATE TABLE IF NOT EXISTS ops.plans (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    monthly_price NUMERIC(10, 2) NOT NULL,
    yearly_price NUMERIC(10, 2) NOT NULL,
    description TEXT NOT NULL,
    features JSONB NOT NULL DEFAULT '[]',
    max_guardians INTEGER NOT NULL DEFAULT 3,
    max_assets INTEGER NOT NULL DEFAULT 10,
    highlighted BOOLEAN NOT NULL DEFAULT false,
    active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed initial plans
INSERT INTO ops.plans (id, name, monthly_price, yearly_price, description, features, max_guardians, max_assets, highlighted, active)
VALUES 
('plan_personal', 'Personal', 19.00, 12.00, 'Secure your individual digital wealth.', 
 '["10 protected assets", "3 active guardians", "1 primary heir", "Standard encryption key"]'::jsonb, 3, 10, false, true),
('plan_family', 'Family', 49.00, 29.00, 'Full protection for your heirs and estates.', 
 '["Unlimited vault assets", "10 active guardians", "5 assigned heirs", "AI Legacy Planner check-in", "Premium support"]'::jsonb, 10, 9999, true, true),
('plan_advisor', 'Advisor', 149.00, 89.00, 'For attorneys, executors, and professional fiduciaries.', 
 '["Unlimited assets & heirs", "Unlimited guardians", "White-labeled customer portal", "Multi-client management", "Dedicated support"]'::jsonb, 999, 99999, false, true)
ON CONFLICT (id) DO UPDATE SET
    name = EXCLUDED.name,
    monthly_price = EXCLUDED.monthly_price,
    yearly_price = EXCLUDED.yearly_price,
    description = EXCLUDED.description,
    features = EXCLUDED.features,
    max_guardians = EXCLUDED.max_guardians,
    max_assets = EXCLUDED.max_assets,
    highlighted = EXCLUDED.highlighted,
    active = EXCLUDED.active,
    updated_at = now();

-- 3. Create ops.coupons table
CREATE TABLE IF NOT EXISTS ops.coupons (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    code TEXT UNIQUE NOT NULL,
    discount_percent INTEGER NOT NULL CHECK (discount_percent > 0 AND discount_percent <= 100),
    max_uses INTEGER NOT NULL DEFAULT 100,
    current_uses INTEGER NOT NULL DEFAULT 0,
    expires_at TIMESTAMPTZ NOT NULL,
    active BOOLEAN NOT NULL DEFAULT true,
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Seed initial coupons
INSERT INTO ops.coupons (code, discount_percent, max_uses, current_uses, expires_at, active, description)
VALUES
('LEGACY50', 50, 100, 0, '2028-12-31T23:59:59Z', true, '50% off Founding Member Launch Discount'),
('ZORVIK25', 25, 500, 0, '2027-12-31T23:59:59Z', true, '25% off Zorvik Tech Partner Promotion'),
('ALPHA2026', 30, 50, 0, '2027-06-30T23:59:59Z', true, '30% Early Access Alpha VIP')
ON CONFLICT (code) DO NOTHING;

-- Grant permissions for ops schema
GRANT ALL ON TABLE ops.user_metadata TO anon, authenticated, service_role;
GRANT ALL ON TABLE ops.plans TO anon, authenticated, service_role;
GRANT ALL ON TABLE ops.coupons TO anon, authenticated, service_role;

NOTIFY pgrst, 'reload schema';

COMMIT;
