BEGIN;

ALTER TABLE audit.events ENABLE ROW LEVEL SECURITY;

DO $$ BEGIN
    CREATE POLICY audit_events_select ON audit.events
        FOR SELECT USING (true);
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

DO $$ BEGIN
    CREATE POLICY audit_events_insert ON audit.events
        FOR INSERT WITH CHECK (true);
EXCEPTION
    WHEN duplicate_object THEN null;
END $$;

CREATE OR REPLACE FUNCTION audit.prevent_mutation()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit events are append-only';
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_audit_events_no_update ON audit.events;
CREATE TRIGGER trg_audit_events_no_update
BEFORE UPDATE ON audit.events
FOR EACH ROW EXECUTE FUNCTION audit.prevent_mutation();

DROP TRIGGER IF EXISTS trg_audit_events_no_delete ON audit.events;
CREATE TRIGGER trg_audit_events_no_delete
BEFORE DELETE ON audit.events
FOR EACH ROW EXECUTE FUNCTION audit.prevent_mutation();

COMMIT;
