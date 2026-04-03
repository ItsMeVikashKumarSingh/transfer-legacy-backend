BEGIN;

-- Persons
DO $$ BEGIN
    CREATE TYPE auth_ext.kyc_status AS ENUM ('unverified', 'pending', 'verified', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS auth_ext.persons (
    person_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    enc_legal_name BYTEA NOT NULL,
    enc_email BYTEA NOT NULL,
    kyc_status auth_ext.kyc_status NOT NULL DEFAULT 'unverified',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE TABLE IF NOT EXISTS auth_ext.person_user_links (
    person_id UUID NOT NULL REFERENCES auth_ext.persons(person_id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    linked_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (person_id, user_id),
    UNIQUE (user_id)
);

-- OPAQUE records
CREATE TABLE IF NOT EXISTS auth_ext.opaque_records (
    user_id UUID PRIMARY KEY REFERENCES auth.users(id) ON DELETE CASCADE,
    opaque_record BYTEA NOT NULL,
    emk_blob BYTEA NOT NULL,
    argon2_params JSONB NOT NULL,
    ed25519_pubkey BYTEA NOT NULL,
    x25519_pubkey BYTEA NOT NULL,
    kyber768_pubkey BYTEA NOT NULL,
    crypto_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

-- Device registry
CREATE TABLE IF NOT EXISTS auth_ext.devices (
    device_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    ed25519_pubkey BYTEA NOT NULL,
    device_meta JSONB NULL,
    last_seen_at TIMESTAMPTZ NULL,
    crypto_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_devices_user_id ON auth_ext.devices(user_id);

-- MFA factors
CREATE TABLE IF NOT EXISTS auth_ext.mfa_factors (
    factor_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    factor_type TEXT NOT NULL,
    totp_secret_enc BYTEA NULL,
    webauthn_credential JSONB NULL,
    enabled BOOLEAN NOT NULL DEFAULT true,
    crypto_version TEXT NOT NULL,
    schema_version INTEGER NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1,
    CONSTRAINT mfa_factor_payload_check
        CHECK (
            (factor_type = 'totp' AND totp_secret_enc IS NOT NULL AND webauthn_credential IS NULL)
            OR
            (factor_type = 'webauthn' AND webauthn_credential IS NOT NULL AND totp_secret_enc IS NULL)
        )
);

CREATE INDEX IF NOT EXISTS idx_mfa_factors_user_id ON auth_ext.mfa_factors(user_id);

-- Step-up challenges
CREATE TABLE IF NOT EXISTS auth_ext.stepup_challenges (
    challenge_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES auth.users(id) ON DELETE CASCADE,
    challenge_type TEXT NOT NULL,
    action TEXT NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL,
    consumed_at TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_by UUID NULL,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_by UUID NULL,
    deleted_at TIMESTAMPTZ NULL,
    deleted_by UUID NULL,
    is_deleted BOOLEAN NOT NULL DEFAULT false,
    version INTEGER NOT NULL DEFAULT 1
);

CREATE INDEX IF NOT EXISTS idx_stepup_challenges_user_id ON auth_ext.stepup_challenges(user_id);
CREATE INDEX IF NOT EXISTS idx_stepup_challenges_expires ON auth_ext.stepup_challenges(expires_at);

COMMIT;
