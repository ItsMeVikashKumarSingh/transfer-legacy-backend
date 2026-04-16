-- Migration: Add label column to inheritance.policies
-- Created: 2026-04-16

BEGIN;

DO $$ 
BEGIN
    IF NOT EXISTS (SELECT 1 FROM information_schema.columns 
                   WHERE table_schema = 'inheritance' 
                   AND table_name = 'policies' 
                   AND column_name = 'label') THEN
        ALTER TABLE inheritance.policies ADD COLUMN label TEXT;
    END IF;
END $$;

-- Update existing policies with a default label
UPDATE inheritance.policies SET label = 'Standard Legacy Plan' WHERE label IS NULL;

COMMIT;
