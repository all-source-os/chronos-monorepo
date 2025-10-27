-- Migration: Tenants
-- Created: 2025-10-27
-- Description: Create tenants table for multi-tenancy support

-- ============================================================================
-- TENANTS TABLE
-- ============================================================================

CREATE TABLE tenants (
    -- Identity
    id VARCHAR(255) PRIMARY KEY,
    name VARCHAR(255) NOT NULL,
    description TEXT,

    -- Quotas (resource limits)
    quota_max_events_per_day BIGINT NOT NULL DEFAULT 1000000,
    quota_max_storage_bytes BIGINT NOT NULL DEFAULT 10737418240,  -- 10 GB
    quota_max_queries_per_hour BIGINT NOT NULL DEFAULT 100000,
    quota_max_api_keys INTEGER NOT NULL DEFAULT 10,
    quota_max_projections INTEGER NOT NULL DEFAULT 50,
    quota_max_pipelines INTEGER NOT NULL DEFAULT 20,

    -- Usage (current resource consumption)
    usage_events_today BIGINT NOT NULL DEFAULT 0,
    usage_total_events BIGINT NOT NULL DEFAULT 0,
    usage_storage_bytes BIGINT NOT NULL DEFAULT 0,
    usage_queries_this_hour BIGINT NOT NULL DEFAULT 0,
    usage_active_api_keys INTEGER NOT NULL DEFAULT 0,
    usage_active_projections INTEGER NOT NULL DEFAULT 0,
    usage_active_pipelines INTEGER NOT NULL DEFAULT 0,
    usage_last_daily_reset TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    usage_last_hourly_reset TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Status
    active BOOLEAN NOT NULL DEFAULT TRUE,

    -- Metadata
    metadata JSONB NOT NULL DEFAULT '{}',

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    CONSTRAINT tenants_name_not_empty CHECK (LENGTH(TRIM(name)) > 0),
    CONSTRAINT tenants_name_max_length CHECK (LENGTH(name) <= 255),
    CONSTRAINT tenants_non_negative_quotas CHECK (
        quota_max_events_per_day >= 0 AND
        quota_max_storage_bytes >= 0 AND
        quota_max_queries_per_hour >= 0 AND
        quota_max_api_keys >= 0 AND
        quota_max_projections >= 0 AND
        quota_max_pipelines >= 0
    ),
    CONSTRAINT tenants_non_negative_usage CHECK (
        usage_events_today >= 0 AND
        usage_total_events >= 0 AND
        usage_storage_bytes >= 0 AND
        usage_queries_this_hour >= 0 AND
        usage_active_api_keys >= 0 AND
        usage_active_projections >= 0 AND
        usage_active_pipelines >= 0
    )
);

-- ============================================================================
-- INDEXES
-- ============================================================================

-- Lookup by name (case-insensitive)
CREATE INDEX idx_tenants_name_lower ON tenants(LOWER(name));

-- Find active tenants
CREATE INDEX idx_tenants_active ON tenants(active, created_at DESC) WHERE active = TRUE;

-- Find tenants by creation date
CREATE INDEX idx_tenants_created_at ON tenants(created_at DESC);

-- JSONB metadata search (GIN for full JSON search)
CREATE INDEX idx_tenants_metadata ON tenants USING GIN (metadata);

-- ============================================================================
-- TRIGGERS
-- ============================================================================

-- Auto-update updated_at timestamp
CREATE OR REPLACE FUNCTION update_tenant_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_tenants_updated_at
    BEFORE UPDATE ON tenants
    FOR EACH ROW
    EXECUTE FUNCTION update_tenant_updated_at();

-- ============================================================================
-- STORED FUNCTIONS
-- ============================================================================

-- Function: Get tenant by name (case-insensitive)
CREATE OR REPLACE FUNCTION get_tenant_by_name(p_name VARCHAR)
RETURNS TABLE (
    id VARCHAR,
    name VARCHAR,
    description TEXT,
    quota_max_events_per_day BIGINT,
    quota_max_storage_bytes BIGINT,
    quota_max_queries_per_hour BIGINT,
    quota_max_api_keys INTEGER,
    quota_max_projections INTEGER,
    quota_max_pipelines INTEGER,
    usage_events_today BIGINT,
    usage_total_events BIGINT,
    usage_storage_bytes BIGINT,
    usage_queries_this_hour BIGINT,
    usage_active_api_keys INTEGER,
    usage_active_projections INTEGER,
    usage_active_pipelines INTEGER,
    usage_last_daily_reset TIMESTAMP WITH TIME ZONE,
    usage_last_hourly_reset TIMESTAMP WITH TIME ZONE,
    active BOOLEAN,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.quota_max_events_per_day,
        t.quota_max_storage_bytes,
        t.quota_max_queries_per_hour,
        t.quota_max_api_keys,
        t.quota_max_projections,
        t.quota_max_pipelines,
        t.usage_events_today,
        t.usage_total_events,
        t.usage_storage_bytes,
        t.usage_queries_this_hour,
        t.usage_active_api_keys,
        t.usage_active_projections,
        t.usage_active_pipelines,
        t.usage_last_daily_reset,
        t.usage_last_hourly_reset,
        t.active,
        t.metadata,
        t.created_at,
        t.updated_at
    FROM tenants t
    WHERE LOWER(t.name) = LOWER(p_name)
    LIMIT 1;
END;
$$ LANGUAGE plpgsql;

-- Function: Get active tenants
CREATE OR REPLACE FUNCTION get_active_tenants(
    p_limit INT DEFAULT 100,
    p_offset INT DEFAULT 0
)
RETURNS TABLE (
    id VARCHAR,
    name VARCHAR,
    description TEXT,
    quota_max_events_per_day BIGINT,
    quota_max_storage_bytes BIGINT,
    quota_max_queries_per_hour BIGINT,
    quota_max_api_keys INTEGER,
    quota_max_projections INTEGER,
    quota_max_pipelines INTEGER,
    usage_events_today BIGINT,
    usage_total_events BIGINT,
    usage_storage_bytes BIGINT,
    usage_queries_this_hour BIGINT,
    usage_active_api_keys INTEGER,
    usage_active_projections INTEGER,
    usage_active_pipelines INTEGER,
    usage_last_daily_reset TIMESTAMP WITH TIME ZONE,
    usage_last_hourly_reset TIMESTAMP WITH TIME ZONE,
    active BOOLEAN,
    metadata JSONB,
    created_at TIMESTAMP WITH TIME ZONE,
    updated_at TIMESTAMP WITH TIME ZONE
) AS $$
BEGIN
    RETURN QUERY
    SELECT
        t.id,
        t.name,
        t.description,
        t.quota_max_events_per_day,
        t.quota_max_storage_bytes,
        t.quota_max_queries_per_hour,
        t.quota_max_api_keys,
        t.quota_max_projections,
        t.quota_max_pipelines,
        t.usage_events_today,
        t.usage_total_events,
        t.usage_storage_bytes,
        t.usage_queries_this_hour,
        t.usage_active_api_keys,
        t.usage_active_projections,
        t.usage_active_pipelines,
        t.usage_last_daily_reset,
        t.usage_last_hourly_reset,
        t.active,
        t.metadata,
        t.created_at,
        t.updated_at
    FROM tenants t
    WHERE t.active = TRUE
    ORDER BY t.created_at DESC
    LIMIT p_limit
    OFFSET p_offset;
END;
$$ LANGUAGE plpgsql;

-- Function: Reset daily usage counters for a tenant
CREATE OR REPLACE FUNCTION reset_tenant_daily_usage(p_tenant_id VARCHAR)
RETURNS VOID AS $$
BEGIN
    UPDATE tenants
    SET usage_events_today = 0,
        usage_last_daily_reset = NOW()
    WHERE id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- Function: Reset hourly usage counters for a tenant
CREATE OR REPLACE FUNCTION reset_tenant_hourly_usage(p_tenant_id VARCHAR)
RETURNS VOID AS $$
BEGIN
    UPDATE tenants
    SET usage_queries_this_hour = 0,
        usage_last_hourly_reset = NOW()
    WHERE id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- Function: Increment event counter
CREATE OR REPLACE FUNCTION increment_tenant_event_count(p_tenant_id VARCHAR)
RETURNS VOID AS $$
BEGIN
    UPDATE tenants
    SET usage_events_today = usage_events_today + 1,
        usage_total_events = usage_total_events + 1
    WHERE id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- Function: Increment query counter
CREATE OR REPLACE FUNCTION increment_tenant_query_count(p_tenant_id VARCHAR)
RETURNS VOID AS $$
BEGIN
    UPDATE tenants
    SET usage_queries_this_hour = usage_queries_this_hour + 1
    WHERE id = p_tenant_id;
END;
$$ LANGUAGE plpgsql;

-- Function: Check if tenant is over quota
CREATE OR REPLACE FUNCTION is_tenant_over_quota(
    p_tenant_id VARCHAR,
    p_resource VARCHAR  -- 'events', 'storage', 'queries', 'api_keys', 'projections', 'pipelines'
)
RETURNS BOOLEAN AS $$
DECLARE
    v_over_quota BOOLEAN := FALSE;
BEGIN
    SELECT CASE p_resource
        WHEN 'events' THEN
            (quota_max_events_per_day > 0 AND usage_events_today >= quota_max_events_per_day)
        WHEN 'storage' THEN
            (quota_max_storage_bytes > 0 AND usage_storage_bytes >= quota_max_storage_bytes)
        WHEN 'queries' THEN
            (quota_max_queries_per_hour > 0 AND usage_queries_this_hour >= quota_max_queries_per_hour)
        WHEN 'api_keys' THEN
            (quota_max_api_keys > 0 AND usage_active_api_keys >= quota_max_api_keys)
        WHEN 'projections' THEN
            (quota_max_projections > 0 AND usage_active_projections >= quota_max_projections)
        WHEN 'pipelines' THEN
            (quota_max_pipelines > 0 AND usage_active_pipelines >= quota_max_pipelines)
        ELSE FALSE
    END INTO v_over_quota
    FROM tenants
    WHERE id = p_tenant_id;

    RETURN COALESCE(v_over_quota, FALSE);
END;
$$ LANGUAGE plpgsql;

-- ============================================================================
-- MONITORING VIEWS
-- ============================================================================

-- Active tenants summary
CREATE VIEW tenant_summary AS
SELECT
    id,
    name,
    active,
    usage_total_events,
    usage_events_today,
    usage_storage_bytes,
    usage_queries_this_hour,
    usage_active_api_keys,
    usage_active_projections,
    usage_active_pipelines,
    quota_max_events_per_day,
    quota_max_storage_bytes,
    quota_max_queries_per_hour,
    created_at,
    updated_at,
    -- Calculate usage percentages (0 = unlimited)
    CASE
        WHEN quota_max_events_per_day = 0 THEN 0
        ELSE (usage_events_today::FLOAT / quota_max_events_per_day * 100)::INT
    END AS events_usage_percent,
    CASE
        WHEN quota_max_storage_bytes = 0 THEN 0
        ELSE (usage_storage_bytes::FLOAT / quota_max_storage_bytes * 100)::INT
    END AS storage_usage_percent,
    CASE
        WHEN quota_max_queries_per_hour = 0 THEN 0
        ELSE (usage_queries_this_hour::FLOAT / quota_max_queries_per_hour * 100)::INT
    END AS queries_usage_percent
FROM tenants
ORDER BY created_at DESC;

-- Tenants approaching quota limits
CREATE VIEW tenants_approaching_limits AS
SELECT
    id,
    name,
    usage_events_today,
    quota_max_events_per_day,
    (usage_events_today::FLOAT / NULLIF(quota_max_events_per_day, 0) * 100)::INT AS events_usage_percent,
    usage_storage_bytes,
    quota_max_storage_bytes,
    (usage_storage_bytes::FLOAT / NULLIF(quota_max_storage_bytes, 0) * 100)::INT AS storage_usage_percent,
    usage_queries_this_hour,
    quota_max_queries_per_hour,
    (usage_queries_this_hour::FLOAT / NULLIF(quota_max_queries_per_hour, 0) * 100)::INT AS queries_usage_percent
FROM tenants
WHERE active = TRUE
  AND (
    (quota_max_events_per_day > 0 AND usage_events_today::FLOAT / quota_max_events_per_day > 0.8) OR
    (quota_max_storage_bytes > 0 AND usage_storage_bytes::FLOAT / quota_max_storage_bytes > 0.8) OR
    (quota_max_queries_per_hour > 0 AND usage_queries_this_hour::FLOAT / quota_max_queries_per_hour > 0.8)
  )
ORDER BY events_usage_percent DESC NULLS LAST;

-- ============================================================================
-- COMMENTS
-- ============================================================================

COMMENT ON TABLE tenants IS 'Multi-tenant configuration and resource management';
COMMENT ON COLUMN tenants.id IS 'Unique tenant identifier (kebab-case)';
COMMENT ON COLUMN tenants.name IS 'Human-readable tenant name';
COMMENT ON COLUMN tenants.description IS 'Optional tenant description';
COMMENT ON COLUMN tenants.quota_max_events_per_day IS 'Maximum events per day (0 = unlimited)';
COMMENT ON COLUMN tenants.quota_max_storage_bytes IS 'Maximum storage in bytes (0 = unlimited)';
COMMENT ON COLUMN tenants.quota_max_queries_per_hour IS 'Maximum queries per hour (0 = unlimited)';
COMMENT ON COLUMN tenants.quota_max_api_keys IS 'Maximum API keys (0 = unlimited)';
COMMENT ON COLUMN tenants.quota_max_projections IS 'Maximum projections (0 = unlimited)';
COMMENT ON COLUMN tenants.quota_max_pipelines IS 'Maximum pipelines (0 = unlimited)';
COMMENT ON COLUMN tenants.usage_events_today IS 'Events ingested today (resets daily)';
COMMENT ON COLUMN tenants.usage_total_events IS 'Total events ingested (lifetime)';
COMMENT ON COLUMN tenants.usage_storage_bytes IS 'Current storage usage in bytes';
COMMENT ON COLUMN tenants.usage_queries_this_hour IS 'Queries this hour (resets hourly)';
COMMENT ON COLUMN tenants.usage_active_api_keys IS 'Number of active API keys';
COMMENT ON COLUMN tenants.usage_active_projections IS 'Number of active projections';
COMMENT ON COLUMN tenants.usage_active_pipelines IS 'Number of active pipelines';
COMMENT ON COLUMN tenants.active IS 'Whether the tenant is active (can ingest/query)';
COMMENT ON COLUMN tenants.metadata IS 'Additional tenant metadata as JSON';

-- ============================================================================
-- GRANTS (adjust as needed for your deployment)
-- ============================================================================

-- Example: Grant read access to app role
-- GRANT SELECT ON tenants TO allsource_app;
-- GRANT SELECT ON tenant_summary TO allsource_app;

-- Example: Grant write access to app role
-- GRANT INSERT, UPDATE, DELETE ON tenants TO allsource_app;

-- ============================================================================
-- SAMPLE QUERIES
-- ============================================================================

-- Get all active tenants:
-- SELECT * FROM get_active_tenants(100, 0);

-- Find tenant by name (case-insensitive):
-- SELECT * FROM get_tenant_by_name('ACME Corp');

-- Check if tenant is over quota:
-- SELECT is_tenant_over_quota('tenant-123', 'events');

-- Reset daily usage for all tenants (run via cron at midnight):
-- UPDATE tenants SET usage_events_today = 0, usage_last_daily_reset = NOW()
-- WHERE usage_last_daily_reset < CURRENT_DATE;

-- Reset hourly usage for all tenants (run via cron hourly):
-- UPDATE tenants SET usage_queries_this_hour = 0, usage_last_hourly_reset = NOW()
-- WHERE usage_last_hourly_reset < DATE_TRUNC('hour', NOW());

-- Get tenant summary:
-- SELECT * FROM tenant_summary WHERE active = TRUE;

-- Get tenants approaching limits:
-- SELECT * FROM tenants_approaching_limits;
