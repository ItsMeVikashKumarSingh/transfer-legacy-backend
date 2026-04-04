BEGIN;

CREATE OR REPLACE FUNCTION inheritance.enforce_policy_transition()
RETURNS TRIGGER AS $$
BEGIN
    IF TG_OP <> 'UPDATE' THEN
        RETURN NEW;
    END IF;

    IF NEW.status = OLD.status THEN
        RETURN NEW;
    END IF;

    IF OLD.status = 'active' AND NEW.status IN ('pending', 'cancelled') THEN
        RETURN NEW;
    ELSIF OLD.status = 'pending' AND NEW.status IN ('active', 'investigating', 'cancelled') THEN
        RETURN NEW;
    ELSIF OLD.status = 'investigating' AND NEW.status IN ('release_ready', 'conflict_pending', 'active') THEN
        RETURN NEW;
    ELSIF OLD.status = 'release_ready' AND NEW.status IN ('released', 'conflict_pending') THEN
        RETURN NEW;
    ELSIF OLD.status = 'conflict_pending' AND NEW.status IN ('manual_review') THEN
        RETURN NEW;
    ELSIF OLD.status = 'manual_review' AND NEW.status IN ('released', 'cancelled') THEN
        RETURN NEW;
    ELSE
        RAISE EXCEPTION 'invalid policy status transition: % -> %', OLD.status, NEW.status;
    END IF;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trg_policy_status_transition ON inheritance.policies;
CREATE TRIGGER trg_policy_status_transition
BEFORE UPDATE OF status ON inheritance.policies
FOR EACH ROW EXECUTE FUNCTION inheritance.enforce_policy_transition();

COMMIT;
