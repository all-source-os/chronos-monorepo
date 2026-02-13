-- 001_initial_schema.sql
-- Initial schema for AllSource Control Plane

CREATE SCHEMA IF NOT EXISTS control_plane;

-- Tenants
CREATE TABLE IF NOT EXISTS control_plane.tenants (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    status      TEXT NOT NULL DEFAULT 'active',
    metadata    JSONB NOT NULL DEFAULT '{}',
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Users
CREATE TABLE IF NOT EXISTS control_plane.users (
    id         TEXT PRIMARY KEY,
    username   TEXT NOT NULL,
    tenant_id  TEXT NOT NULL REFERENCES control_plane.tenants(id),
    role       TEXT NOT NULL DEFAULT 'ReadOnly',
    is_api_key BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Policies
CREATE TABLE IF NOT EXISTS control_plane.policies (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    resource    TEXT NOT NULL,
    action      TEXT NOT NULL,
    conditions  JSONB NOT NULL DEFAULT '[]',
    priority    INTEGER NOT NULL DEFAULT 0,
    enabled     BOOLEAN NOT NULL DEFAULT TRUE,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Audit Events
CREATE TABLE IF NOT EXISTS control_plane.audit_events (
    id          BIGINT GENERATED ALWAYS AS IDENTITY PRIMARY KEY,
    timestamp   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type  TEXT NOT NULL,
    user_id     TEXT NOT NULL DEFAULT '',
    username    TEXT NOT NULL DEFAULT '',
    tenant_id   TEXT NOT NULL DEFAULT '',
    action      TEXT NOT NULL,
    resource    TEXT NOT NULL DEFAULT '',
    resource_id TEXT NOT NULL DEFAULT '',
    method      TEXT NOT NULL DEFAULT '',
    path        TEXT NOT NULL DEFAULT '',
    status_code INTEGER NOT NULL DEFAULT 0,
    duration    DOUBLE PRECISION NOT NULL DEFAULT 0,
    ip_address  TEXT NOT NULL DEFAULT '',
    user_agent  TEXT NOT NULL DEFAULT '',
    error       TEXT NOT NULL DEFAULT '',
    metadata    JSONB NOT NULL DEFAULT '{}'
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_audit_user ON control_plane.audit_events (user_id);
CREATE INDEX IF NOT EXISTS idx_audit_tenant ON control_plane.audit_events (tenant_id);
CREATE INDEX IF NOT EXISTS idx_audit_ts ON control_plane.audit_events (timestamp);
CREATE INDEX IF NOT EXISTS idx_policies_resource ON control_plane.policies (resource);
CREATE INDEX IF NOT EXISTS idx_users_tenant ON control_plane.users (tenant_id);

-- Seed default tenant
INSERT INTO control_plane.tenants (id, name, description)
VALUES ('default', 'Default Tenant', 'The default system tenant')
ON CONFLICT DO NOTHING;

-- Seed default policies (reconciled from policy.go — all 5 defaults)

-- Policy 1: Prevent deletion of default tenant
INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority)
VALUES (
    'prevent-default-tenant-deletion',
    'Prevent Default Tenant Deletion',
    'Prevents deletion of the default tenant',
    'tenant',
    'deny',
    '[{"field": "tenant_id", "operator": "eq", "value": "default"}, {"field": "operation", "operator": "eq", "value": "delete"}]',
    100
) ON CONFLICT DO NOTHING;

-- Policy 2: Require admin for tenant creation
INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority)
VALUES (
    'require-admin-tenant-create',
    'Require Admin for Tenant Creation',
    'Only admins can create new tenants',
    'tenant',
    'deny',
    '[{"field": "operation", "operator": "eq", "value": "create"}, {"field": "role", "operator": "ne", "value": "Admin"}]',
    90
) ON CONFLICT DO NOTHING;

-- Policy 3: Warn on large operations
INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority)
VALUES (
    'warn-large-operations',
    'Warn on Large Operations',
    'Warn when operations affect more than 10000 records',
    'operation',
    'warn',
    '[{"field": "record_count", "operator": "gt", "value": 10000}]',
    50
) ON CONFLICT DO NOTHING;

-- Policy 4: Prevent self-deletion
INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority)
VALUES (
    'prevent-self-deletion',
    'Prevent Self Deletion',
    'Users cannot delete themselves',
    'user',
    'deny',
    '[{"field": "operation", "operator": "eq", "value": "delete"}, {"field": "target_user_id", "operator": "eq", "value": "${user_id}"}]',
    95
) ON CONFLICT DO NOTHING;

-- Policy 5: Rate limit expensive operations
INSERT INTO control_plane.policies (id, name, description, resource, action, conditions, priority)
VALUES (
    'rate-limit-expensive-ops',
    'Rate Limit Expensive Operations',
    'Limit snapshot and backup operations',
    'operation',
    'deny',
    '[{"field": "operation_type", "operator": "in", "value": ["snapshot", "backup", "restore"]}, {"field": "recent_operations", "operator": "gt", "value": 5}]',
    80
) ON CONFLICT DO NOTHING;
