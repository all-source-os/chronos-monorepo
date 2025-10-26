-- Event Streams Schema for PostgreSQL
-- Phase 4B: Persistent Storage
-- Based on SierraDB patterns with partition-aware design

-- Event Streams table
-- Stores stream metadata including version, watermark, and partition assignment
CREATE TABLE IF NOT EXISTS event_streams (
    stream_id VARCHAR(255) PRIMARY KEY,
    partition_id INTEGER NOT NULL,
    current_version BIGINT NOT NULL DEFAULT 0,
    watermark BIGINT NOT NULL DEFAULT 0,
    expected_version BIGINT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Constraints
    CHECK (current_version >= 0),
    CHECK (watermark >= 0),
    CHECK (watermark <= current_version),
    CHECK (partition_id >= 0 AND partition_id < 32)
);

-- Events table
-- Stores individual events within streams
CREATE TABLE IF NOT EXISTS events (
    id BIGSERIAL PRIMARY KEY,
    stream_id VARCHAR(255) NOT NULL REFERENCES event_streams(stream_id) ON DELETE CASCADE,
    version BIGINT NOT NULL,
    tenant_id VARCHAR(255) NOT NULL,
    event_type VARCHAR(255) NOT NULL,
    entity_id VARCHAR(255) NOT NULL,
    payload JSONB NOT NULL,
    metadata JSONB,
    timestamp TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Ensure unique versions within a stream
    UNIQUE(stream_id, version),

    -- Constraints
    CHECK (version > 0)
);

-- Indexes for high-performance queries
CREATE INDEX IF NOT EXISTS idx_events_stream_version ON events(stream_id, version);
CREATE INDEX IF NOT EXISTS idx_events_stream_id ON events(stream_id);
CREATE INDEX IF NOT EXISTS idx_events_tenant ON events(tenant_id);
CREATE INDEX IF NOT EXISTS idx_events_entity ON events(entity_id);
CREATE INDEX IF NOT EXISTS idx_events_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_stream_partition ON event_streams(partition_id);

-- Partial index for active streams (those with events)
CREATE INDEX IF NOT EXISTS idx_stream_active ON event_streams(partition_id, current_version)
WHERE current_version > 0;

-- Partition statistics view
-- Provides real-time view of partition distribution
CREATE OR REPLACE VIEW partition_stats AS
SELECT
    partition_id,
    COUNT(*) as stream_count,
    SUM(current_version) as total_events,
    AVG(current_version) as avg_events_per_stream,
    MAX(current_version) as max_events_in_stream,
    MIN(updated_at) as oldest_update,
    MAX(updated_at) as newest_update
FROM event_streams
GROUP BY partition_id
ORDER BY partition_id;

-- Stream health view
-- Identifies streams with potential issues
CREATE OR REPLACE VIEW stream_health AS
SELECT
    stream_id,
    partition_id,
    current_version,
    watermark,
    (current_version - watermark) as gap_size,
    updated_at,
    CASE
        WHEN watermark < current_version THEN 'HAS_GAPS'
        WHEN current_version = 0 THEN 'EMPTY'
        ELSE 'HEALTHY'
    END as health_status
FROM event_streams
WHERE current_version != watermark
ORDER BY (current_version - watermark) DESC;

-- Function to verify gapless versions
-- Returns true if all versions from 1 to watermark exist
CREATE OR REPLACE FUNCTION verify_stream_gapless(p_stream_id VARCHAR)
RETURNS BOOLEAN AS $$
DECLARE
    v_watermark BIGINT;
    v_gap_count BIGINT;
BEGIN
    -- Get watermark
    SELECT watermark INTO v_watermark
    FROM event_streams
    WHERE stream_id = p_stream_id;

    IF v_watermark IS NULL THEN
        RETURN TRUE; -- Stream doesn't exist or watermark is 0
    END IF;

    -- Check for gaps using generate_series
    SELECT COUNT(*) INTO v_gap_count
    FROM generate_series(1, v_watermark) AS expected_version
    LEFT JOIN events ON events.stream_id = p_stream_id
        AND events.version = expected_version
    WHERE events.version IS NULL;

    RETURN (v_gap_count = 0);
END;
$$ LANGUAGE plpgsql;

-- Trigger to automatically update updated_at timestamp
CREATE OR REPLACE FUNCTION update_event_stream_timestamp()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER event_stream_update_timestamp
BEFORE UPDATE ON event_streams
FOR EACH ROW
EXECUTE FUNCTION update_event_stream_timestamp();

-- Comments for documentation
COMMENT ON TABLE event_streams IS 'Event stream metadata with SierraDB partition-aware design';
COMMENT ON TABLE events IS 'Individual events within streams, gapless version guarantee';
COMMENT ON COLUMN event_streams.watermark IS 'Highest continuously confirmed version (gapless guarantee)';
COMMENT ON COLUMN event_streams.expected_version IS 'Expected version for optimistic locking';
COMMENT ON COLUMN event_streams.partition_id IS 'Fixed partition ID (0-31 for single-node)';
COMMENT ON VIEW partition_stats IS 'Real-time partition distribution statistics';
COMMENT ON VIEW stream_health IS 'Stream health monitoring with gap detection';
