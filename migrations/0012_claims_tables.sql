BEGIN;

DO $$ BEGIN
    CREATE TYPE inheritance.claim_type AS ENUM ('type_a', 'type_b');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE inheritance.claim_status AS ENUM (
        'pending_confirmation',
        'confirmed',
        'rejected',
        'cancelled'
    );
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE inheritance.attachment_status AS ENUM ('pending', 'confirmed', 'rejected');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE TYPE inheritance.signature_type AS ENUM ('ed25519', 'dilithium2');
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE TABLE IF NOT EXISTS inheritance.claims (
    claim_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    claimant_person_id UUID NOT NULL REFERENCES auth_ext.persons(person_id) ON DELETE CASCADE,
    claim_type inheritance.claim_type NOT NULL,
    status inheritance.claim_status NOT NULL DEFAULT 'pending_confirmation',
    confirmation_deadline TIMESTAMPTZ NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMPTZ NULL,
    rejected_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_claims_policy_id ON inheritance.claims(policy_id);
CREATE INDEX IF NOT EXISTS idx_claims_person_id ON inheritance.claims(claimant_person_id);

CREATE TABLE IF NOT EXISTS inheritance.claim_attachments (
    attachment_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    claim_id UUID NOT NULL REFERENCES inheritance.claims(claim_id) ON DELETE CASCADE,
    object_key TEXT NOT NULL,
    sha256 BYTEA NULL,
    size_bytes BIGINT NULL,
    mime_type TEXT NULL,
    status inheritance.attachment_status NOT NULL DEFAULT 'pending',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMPTZ NULL
);

CREATE INDEX IF NOT EXISTS idx_claim_attachments_claim_id ON inheritance.claim_attachments(claim_id);

CREATE TABLE IF NOT EXISTS inheritance.attestations (
    attestation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    policy_id UUID NOT NULL REFERENCES inheritance.policies(policy_id) ON DELETE CASCADE,
    claim_id UUID NOT NULL REFERENCES inheritance.claims(claim_id) ON DELETE CASCADE,
    approver_person_id UUID NOT NULL REFERENCES auth_ext.persons(person_id) ON DELETE CASCADE,
    statement JSONB NOT NULL,
    signature BYTEA NOT NULL,
    signature_type inheritance.signature_type NOT NULL DEFAULT 'ed25519',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_attestations_claim_id ON inheritance.attestations(claim_id);

COMMIT;
