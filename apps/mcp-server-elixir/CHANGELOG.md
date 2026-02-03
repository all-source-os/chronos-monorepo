# Changelog

All notable changes to the MCP Server will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-02-03

### Added

#### AI-Native Enhancements - Complete

**Enhanced Tool Descriptions (US-013)**
- Rich tool descriptions with agent guidance
- Best practices and common patterns embedded in descriptions
- Performance tips for each tool
- Decision trees for tool selection
- Context-aware recommendations

**Query Advice Tool (US-014)**
- `get_query_advice` tool implementation
- Use-case specific recommendations
- Query pattern suggestions based on data shape
- Performance optimization tips
- Index usage recommendations

**Conversation Context Manager (US-015)**
- `ConversationContext` for multi-turn interactions
- Session-based query refinement
- Iterative query composition
- Context preservation across tool calls
- Memory of previous queries and results

**Quick Exploration Tools (US-016)**
- `sample_events` for fast data exploration
- `quick_stats` for rapid statistics
- Stratified sampling options
- Random and time-based sampling modes
- Configurable sample sizes

#### Native Search Integration

**Semantic Search Tools**
- `semantic_search_events` - Vector similarity search
- `hybrid_search` - Combined vector + keyword search
- Configurable similarity thresholds
- Metadata filtering support

#### Real-Time Integration

**Core WebSocket Client**
- `CoreWebSocketClient` for real-time Core events
- Auto-reconnect with exponential backoff
- PubSub integration for event distribution
- Connection health monitoring

**Broadway Pipeline**
- `CoreProducer` for Broadway integration
- `EventPipeline` for stream processing
- Cursor tracking and persistence
- Backpressure handling

**Projection Sync**
- `ProjectionSync` GenServer
- ETS cache for local reads
- Automatic state restoration

### Changed
- Updated all tool descriptions with AI-native guidance
- Enhanced error messages for better agent understanding
- Improved response formatting for LLM consumption

### Technical Details
- 4 new AI-native tools
- 2 new search tools
- Real-time event streaming infrastructure
- ~2500 LOC added

---

## [0.1.0] - 2025-12-01

### Added

#### MCP v2.0 Phase 1 - Complete

**Advanced Query Tools**
- `advanced_query` - Complex queries with aggregations
- `time_series_analysis` - Trend analysis over time

**Analytics Tools**
- `funnel_analysis` - Conversion tracking
- `detect_anomalies` - Real-time anomaly detection

**Projection Tools**
- `create_projection` - Materialized views
- `get_projection_state` - Query projections
- `list_projections` - List all projections

**Pipeline Tools**
- `execute_pipeline` - Event processing
- `replay_events` - Event replay
- `validate_events` - Data validation

**Policy Tools**
- `create_policy` - Policy creation
- `evaluate_policy` - Policy evaluation
- `list_policies` - List policies

### Technical Details
- 13 advanced tools implemented
- ~900 LOC
- Full MCP protocol compliance

---

## Version History

| Version | Date | Status | Highlights |
|---------|------|--------|------------|
| [0.2.0] | 2026-02-03 | Current | AI-Native, Search, Real-Time Integration |
| [0.1.0] | 2025-12-01 | Stable | MCP v2.0 Phase 1, 13 tools |
