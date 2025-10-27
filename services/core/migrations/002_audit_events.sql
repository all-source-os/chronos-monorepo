-- Migration: Audit Events
-- Created: 2025-10-26
-- Description: Create audit_events table for security, compliance, and debugging

-- ============================================================================
-- AUDIT EVENTS TABLE
-- ============================================================================

CREATE TABLE audit_events (
    -- Identity
    id UUID PRIMARY KEY,
    tenant_id VARCHAR(255) NOT NULL,

    -- Timestamp
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Action
    action VARCHAR(100) NOT NULL,
    category VARCHAR(50) NOT NULL,

    -- Actor (who performed the action)
    actor_type VARCHAR(20) NOT NULL CHECK (actor_type IN ('user', 'api_key', 'system')),
    actor_id VARCHAR(255) NOT NULL,
    actor_name VARCHAR(255) NOT NULL,

    -- Resource (what was affected)
    resource_type VARCHAR(100),
    resource_id VARCHAR(255),

    -- Outcome
    outcome VARCHAR(20) NOT NULL CHECK (outcome IN ('success', 'failure', 'partial_success')),

    -- Context
    ip_address INET,
    user_agent TEXT,
    request_id VARCHAR(100),

    -- Error details (if failure)
    error_message TEXT,

    -- Additional metadata (JSON)
    metadata JSONB,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- INDEXES
-- ============================================================================

-- Primary lookup indexes
CREATE INDEX idx_audit_events_tenant_timestamp ON audit_events(tenant_id, timestamp DESC);
CREATE INDEX idx_audit_events_tenant_action ON audit_events(tenant_id, action);
CREATE INDEX idx_audit_events_tenant_actor ON audit_events(tenant_id, actor_id);

-- Security monitoring indexes
CREATE INDEX idx_audit_events_security ON audit_events(tenant_id, timestamp DESC)
    WHERE action IN ('login_failed', 'permission_denied', 'rate_limit_exceeded', 'ip_blocked', 'suspicious_activity');

-- Resource tracking
CREATE INDEX idx_audit_events_resource ON audit_events(tenant_id, resource_type, resource_id);

-- Outcome tracking
CREATE INDEX idx_audit_events_failures ON audit_events(tenant_id, timestamp DESC)
    WHERE outcome = 'failure';

-- JSONB metadata index (GIN for full JSON search)
CREATE INDEX idx_audit_events_metadata ON audit_events USING GIN (metadata);

-- ============================================================================
-- PARTITIONING (Optional - for high-volume deployments)
-- ============================================================================

-- Note: For production systems with >1M audit events/day, consider:
-- 1. Partitioning by timestamp (monthly or weekly)
-- 2. Separate tablespace for audit data
-- 3. Archival strategy for old events

-- Example partition setup (commented out by default):
-- CREATE TABLE audit_events_2025_10 PARTITION OF audit_events
--     FOR VALUES FROM ('2025-10-01') TO ('2025-11-01');

-- ============================================================================
-- MONITORING VIEWS
-- ============================================================================

-- Security events view (failed logins, permission denials, etc.)
CREATE VIEW audit_security_events AS
SELECT
    id,
    tenant_id,
    timestamp,
    action,
    actor_type,
    actor_id,
    actor_name,
    ip_address,
    user_agent,
    error_message
FROM audit_events
WHERE action IN ('login_failed', 'permission_denied', 'rate_limit_exceeded', 'ip_blocked', 'suspicious_activity')
ORDER BY timestamp DESC;

-- Recent failures view
CREATE VIEW audit_recent_failures AS
SELECT
    tenant_id,
    action,
    actor_id,
    actor_name,
    COUNT(*) as failure_count,
    MAX(timestamp) as last_failure,
    array_agg(DISTINCT ip_address) as ip_addresses
FROM audit_events
WHERE outcome = 'failure'
  AND timestamp > NOW() - INTERVAL '24 hours'
GROUP BY tenant_id, action, actor_id, actor_name
ORDER BY failure_count DESC;

-- Tenant activity summary
CREATE VIEW audit_tenant_activity AS
SELECT
    tenant_id,
    DATE(timestamp) as activity_date,
    COUNT(*) as total_events,
    COUNT(*) FILTER (WHERE outcome = 'success') as successful_events,
    COUNT(*) FILTER (WHERE outcome = 'failure') as failed_events,
    COUNT(DISTINCT actor_id) as unique_actors,
    array_agg(DISTINCT action) as actions_performed
FROM audit_events
WHERE timestamp > NOW() - INTERVAL '30 days'
GROUP BY tenant_id, DATE(timestamp)
ORDER BY activity_date DESC, tenant_id;

-- ============================================================================
-- STORED FUNCTIONS
-- ============================================================================

-- Function: Get audit events for a tenant with filters
CREATE OR REPLACE FUNCTION get_audit_events(
    p_tenant_id VARCHAR,
    p_start_time TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    p_end_time TIMESTAMP WITH TIME ZONE DEFAULT NULL,
    p_action VARCHAR DEFAULT NULL,
    p_actor_id VARCHAR DEFAULT NULL,
    p_limit INT DEFAULT 100,
    p_offset INT DEFAULT 0
)
RETURNS TABLE (
    id UUID,
    tenant_id VARCHAR,
    timestamp TIMESTAMP WITH TIME ZONE,
    action VARCHAR,
    actor_type VARCHAR,
    actor_id VARCHAR,
    actor_name VARCHAR,
    resource_type VARCHAR,
    resource_id VARCHAR,
    outcome VARCHAR,
    ip_address INET,
    error_message TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        ae.id,
        ae.tenant_id,
        ae.timestamp,
        ae.action,
        ae.actor_type,
        ae.actor_id,
        ae.actor_name,
        ae.resource_type,
        ae.resource_id,
        ae.outcome,
        ae.ip_address,
        ae.error_message
    FROM audit_events ae
    WHERE ae.tenant_id = p_tenant_id
      AND (p_start_time IS NULL OR ae.timestamp >= p_start_time)
      AND (p_end_time IS NULL OR ae.timestamp <= p_end_time)
      AND (p_action IS NULL OR ae.action = p_action)
      AND (p_actor_id IS NULL OR ae.actor_id = p_actor_id)
    ORDER BY ae.timestamp DESC
    LIMIT p_limit
    OFFSET p_offset;
END;
$$ LANGUAGE plpgsql;

-- Function: Get security events for a tenant
CREATE OR REPLACE FUNCTION get_security_events(
    p_tenant_id VARCHAR,
    p_hours INT DEFAULT 24,
    p_limit INT DEFAULT 100
)
RETURNS TABLE (
    id UUID,
    timestamp TIMESTAMP WITH TIME ZONE,
    action VARCHAR,
    actor_id VARCHAR,
    actor_name VARCHAR,
    ip_address INET,
    error_message TEXT
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        ae.id,
        ae.timestamp,
        ae.action,
        ae.actor_id,
        ae.actor_name,
        ae.ip_address,
        ae.error_message
    FROM audit_events ae
    WHERE ae.tenant_id = p_tenant_id
      AND ae.timestamp > NOW() - (p_hours || ' hours')::INTERVAL
      AND ae.action IN ('login_failed', 'permission_denied', 'rate_limit_exceeded', 'ip_blocked', 'suspicious_activity')
    ORDER BY ae.timestamp DESC
    LIMIT p_limit;
END;
$$ LANGUAGE plpgsql;

-- Function: Purge old audit events (for GDPR compliance)
CREATE OR REPLACE FUNCTION purge_old_audit_events(
    p_tenant_id VARCHAR,
    p_older_than TIMESTAMP WITH TIME ZONE
)
RETURNS INT AS $$
DECLARE
    v_deleted_count INT;
BEGIN
    DELETE FROM audit_events
    WHERE tenant_id = p_tenant_id
      AND timestamp < p_older_than;

    GET DIAGNOSTICS v_deleted_count = ROW_COUNT;

    RETURN v_deleted_count;
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Auto-update created_at timestamp
CREATE OR REPLACE FUNCTION update_audit_created_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.created_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_audit_events_created_at
    BEFORE INSERT ON audit_events
    FOR EACH ROW
    EXECUTE FUNCTION update_audit_created_at();

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE audit_events IS 'Immutable audit log for all system operations (security, compliance, debugging)';
COMMENT ON COLUMN audit_events.id IS 'Unique UUID for each audit event';
COMMENT ON COLUMN audit_events.tenant_id IS 'Tenant this audit event belongs to (for isolation)';
COMMENT ON COLUMN audit_events.timestamp IS 'When the action occurred (UTC)';
COMMENT ON COLUMN audit_events.action IS 'Action performed (login, event_ingested, etc.)';
COMMENT ON COLUMN audit_events.category IS 'Category of action (authentication, event, security, etc.)';
COMMENT ON COLUMN audit_events.actor_type IS 'Type of actor (user, api_key, system)';
COMMENT ON COLUMN audit_events.actor_id IS 'Identifier of the actor';
COMMENT ON COLUMN audit_events.actor_name IS 'Display name of the actor';
COMMENT ON COLUMN audit_events.resource_type IS 'Type of resource affected (event_stream, schema, etc.)';
COMMENT ON COLUMN audit_events.resource_id IS 'Identifier of the resource';
COMMENT ON COLUMN audit_events.outcome IS 'Result of the action (success, failure, partial_success)';
COMMENT ON COLUMN audit_events.ip_address IS 'IP address of the requester';
COMMENT ON COLUMN audit_events.user_agent IS 'User agent of the requester';
COMMENT ON COLUMN audit_events.request_id IS 'Request ID for correlation across services';
COMMENT ON COLUMN audit_events.error_message IS 'Error message if action failed';
COMMENT ON COLUMN audit_events.metadata IS 'Additional metadata as JSON';

-- ============================================================================
-- GRANTS (adjust as needed for your deployment)
-- ============================================================================

-- Example: Grant read access to audit_read role
-- GRANT SELECT ON audit_events TO audit_read;
-- GRANT SELECT ON audit_security_events TO audit_read;
-- GRANT SELECT ON audit_recent_failures TO audit_read;

-- Example: Grant write access to application role
-- GRANT INSERT ON audit_events TO allsource_app;

-- ============================================================================
-- SAMPLE QUERIES
-- ============================================================================

-- Get recent security events for a tenant:
-- SELECT * FROM get_security_events('tenant-123', 24, 100);

-- Get all audit events for a specific user:
-- SELECT * FROM get_audit_events('tenant-123', NULL, NULL, NULL, 'user:john-doe', 100, 0);

-- Get failed login attempts in the last hour:
-- SELECT * FROM audit_events
-- WHERE tenant_id = 'tenant-123'
--   AND action = 'login_failed'
--   AND timestamp > NOW() - INTERVAL '1 hour'
-- ORDER BY timestamp DESC;

-- Get activity summary for a tenant:
-- SELECT * FROM audit_tenant_activity WHERE tenant_id = 'tenant-123';

-- Purge events older than 90 days:
-- SELECT purge_old_audit_events('tenant-123', NOW() - INTERVAL '90 days');
