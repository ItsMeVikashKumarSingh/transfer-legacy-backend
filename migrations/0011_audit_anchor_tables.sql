BEGIN;

CREATE TABLE IF NOT EXISTS audit.anchors (
    anchor_date DATE PRIMARY KEY,
    head_hash BYTEA NOT NULL,
    entry_count INTEGER NOT NULL,
    snapshot JSONB NOT NULL,
    signature TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

COMMIT;
