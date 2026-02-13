-- 002_operations.sql
-- Operations table for tracking long-running operations

CREATE TABLE IF NOT EXISTS control_plane.operations (
    id           TEXT PRIMARY KEY,
    type         TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'pending',
    tenant_id    TEXT NOT NULL DEFAULT '',
    parameters   JSONB NOT NULL DEFAULT '{}',
    result       JSONB NOT NULL DEFAULT '{}',
    initiated_by TEXT NOT NULL DEFAULT '',
    started_at   TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    error        TEXT NOT NULL DEFAULT '',
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_ops_type ON control_plane.operations (type);
CREATE INDEX IF NOT EXISTS idx_ops_status ON control_plane.operations (status);
CREATE INDEX IF NOT EXISTS idx_ops_created ON control_plane.operations (created_at);
