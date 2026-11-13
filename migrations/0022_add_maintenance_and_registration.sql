-- Migration: Add maintenance mode and registration enabled fields to app.settings
BEGIN;

ALTER TABLE app.settings 
ADD COLUMN IF NOT EXISTS maintenance_mode BOOLEAN NOT NULL DEFAULT FALSE,
ADD COLUMN IF NOT EXISTS registration_enabled BOOLEAN NOT NULL DEFAULT TRUE;

-- Update the existing config row to ensure values are set
UPDATE app.settings 
SET maintenance_mode = FALSE, registration_enabled = TRUE 
WHERE id = 1;

COMMIT;
