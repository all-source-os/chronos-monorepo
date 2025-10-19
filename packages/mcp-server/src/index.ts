#!/usr/bin/env node

/**
 * AllSource MCP Server - AI-Native Event Store Interface
 *
 * This MCP server enables Large Language Models to interact with AllSource's
 * temporal event store through natural language, providing unprecedented access
 * to historical data with time-travel capabilities.
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from '@modelcontextprotocol/sdk/types.js';
import axios from 'axios';
import { z } from 'zod';

const CORE_API_URL = process.env.ALLSOURCE_CORE_URL || 'http://localhost:8080';
const CONTROL_PLANE_URL = process.env.ALLSOURCE_CONTROL_URL || 'http://localhost:8081';

// Zod schemas for validation
const QueryEventsSchema = z.object({
  entity_id: z.string().optional(),
  event_type: z.string().optional(),
  as_of: z.string().optional(),
  since: z.string().optional(),
  until: z.string().optional(),
  limit: z.number().optional(),
});

const ReconstructStateSchema = z.object({
  entity_id: z.string(),
  as_of: z.string().optional(),
});

const IngestEventSchema = z.object({
  event_type: z.string(),
  entity_id: z.string(),
  payload: z.record(z.any()),
  metadata: z.record(z.any()).optional(),
});

const AnalyzeChangesSchema = z.object({
  entity_id: z.string(),
  from_time: z.string(),
  to_time: z.string().optional(),
});

const FindPatternsSchema = z.object({
  entity_id: z.string().optional(),
  event_type: z.string().optional(),
  since: z.string().optional(),
  pattern_type: z.enum(['frequency', 'sequence', 'anomaly']).optional(),
});

const CompareEntitiesSchema = z.object({
  entity_ids: z.array(z.string()),
  timeframe: z.string().optional(),
});

const EventTimelineSchema = z.object({
  entity_id: z.string(),
  since: z.string().optional(),
  until: z.string().optional(),
});

// Define MCP tools with enhanced capabilities
const tools: Tool[] = [
  {
    name: 'query_events',
    description: 'Query events with flexible filters. Use natural language timeframes like "since yesterday" and the LLM will convert them to ISO timestamps.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'Filter by entity ID (e.g., "user-123")' },
        event_type: { type: 'string', description: 'Filter by event type (e.g., "user.created")' },
        as_of: { type: 'string', description: 'Time-travel: get events as of this ISO timestamp' },
        since: { type: 'string', description: 'Get events since this ISO timestamp' },
        until: { type: 'string', description: 'Get events until this ISO timestamp' },
        limit: { type: 'number', description: 'Limit number of results (default: all)' },
      },
    },
  },
  {
    name: 'reconstruct_state',
    description: 'Reconstruct the complete state of an entity at any point in time by replaying its event stream. Perfect for answering "What did this entity look like on date X?"',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'The entity ID to reconstruct state for' },
        as_of: { type: 'string', description: 'Reconstruct state as of this ISO timestamp (optional, defaults to current)' },
      },
      required: ['entity_id'],
    },
  },
  {
    name: 'get_snapshot',
    description: 'Get the current snapshot of an entity (much faster than reconstruction). Use this when you need the latest state without time-travel.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'The entity ID to get snapshot for' },
      },
      required: ['entity_id'],
    },
  },
  {
    name: 'analyze_changes',
    description: 'Analyze what changed for an entity between two points in time. Returns a detailed diff showing added, modified, and removed fields.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'The entity to analyze' },
        from_time: { type: 'string', description: 'Start timestamp (ISO format)' },
        to_time: { type: 'string', description: 'End timestamp (ISO format, defaults to now)' },
      },
      required: ['entity_id', 'from_time'],
    },
  },
  {
    name: 'find_patterns',
    description: 'Detect patterns in event streams: frequency analysis, event sequences, or anomalies. Perfect for answering "What unusual patterns exist?"',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'Analyze patterns for specific entity (optional)' },
        event_type: { type: 'string', description: 'Analyze patterns for specific event type (optional)' },
        since: { type: 'string', description: 'Analyze patterns since this timestamp (optional)' },
        pattern_type: {
          type: 'string',
          enum: ['frequency', 'sequence', 'anomaly'],
          description: 'Type of pattern to detect (frequency=event counts, sequence=event order, anomaly=unusual events)'
        },
      },
    },
  },
  {
    name: 'compare_entities',
    description: 'Compare multiple entities to find similarities and differences in their event histories.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Array of entity IDs to compare'
        },
        timeframe: { type: 'string', description: 'Compare within this timeframe (ISO timestamp)' },
      },
      required: ['entity_ids'],
    },
  },
  {
    name: 'event_timeline',
    description: 'Get a chronological timeline of all events for an entity, formatted for easy reading and understanding.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'Entity to get timeline for' },
        since: { type: 'string', description: 'Timeline start time (optional)' },
        until: { type: 'string', description: 'Timeline end time (optional)' },
      },
      required: ['entity_id'],
    },
  },
  {
    name: 'explain_entity',
    description: 'Get a comprehensive explanation of an entity: current state, event history, key changes, and timeline summary.',
    inputSchema: {
      type: 'object',
      properties: {
        entity_id: { type: 'string', description: 'Entity ID to explain' },
      },
      required: ['entity_id'],
    },
  },
  {
    name: 'ingest_event',
    description: 'Ingest a new event into the AllSource event store.',
    inputSchema: {
      type: 'object',
      properties: {
        event_type: { type: 'string', description: 'Type of event (e.g., "user.created")' },
        entity_id: { type: 'string', description: 'ID of the entity this event relates to' },
        payload: { type: 'object', description: 'Event payload as JSON object' },
        metadata: { type: 'object', description: 'Optional metadata' },
      },
      required: ['event_type', 'entity_id', 'payload'],
    },
  },
  {
    name: 'get_stats',
    description: 'Get comprehensive statistics about the AllSource event store.',
    inputSchema: {
      type: 'object',
      properties: {},
    },
  },
  {
    name: 'get_cluster_status',
    description: 'Get current cluster health and status information.',
    inputSchema: {
      type: 'object',
      properties: {},
    },
  },
];

class AllSourceMCPServer {
  private server: Server;

  constructor() {
    this.server = new Server(
      {
        name: 'allsource-mcp',
        version: '0.1.0',
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.setupHandlers();
  }

  private setupHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools,
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          case 'query_events':
            return await this.queryEvents(args);
          case 'reconstruct_state':
            return await this.reconstructState(args);
          case 'get_snapshot':
            return await this.getSnapshot(args);
          case 'analyze_changes':
            return await this.analyzeChanges(args);
          case 'find_patterns':
            return await this.findPatterns(args);
          case 'compare_entities':
            return await this.compareEntities(args);
          case 'event_timeline':
            return await this.eventTimeline(args);
          case 'explain_entity':
            return await this.explainEntity(args);
          case 'ingest_event':
            return await this.ingestEvent(args);
          case 'get_stats':
            return await this.getStats();
          case 'get_cluster_status':
            return await this.getClusterStatus();
          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          content: [
            {
              type: 'text',
              text: `❌ Error: ${errorMessage}`,
            },
          ],
        };
      }
    });
  }

  private async queryEvents(args: unknown) {
    const params = QueryEventsSchema.parse(args);
    const response = await axios.get(`${CORE_API_URL}/api/v1/events/query`, { params });

    const data = response.data;
    const summary = `📊 Found ${data.count} events\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(data, null, 2),
        },
      ],
    };
  }

  private async reconstructState(args: unknown) {
    const { entity_id, as_of } = ReconstructStateSchema.parse(args);
    const url = `${CORE_API_URL}/api/v1/entities/${entity_id}/state`;
    const params = as_of ? { as_of } : {};

    const response = await axios.get(url, { params });

    const state = response.data;
    const summary = `🔄 Reconstructed state for "${entity_id}"\n` +
      `📅 As of: ${state.as_of || 'current'}\n` +
      `📊 Events processed: ${state.event_count}\n` +
      `⏰ Last updated: ${state.last_updated}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(state, null, 2),
        },
      ],
    };
  }

  private async getSnapshot(args: unknown) {
    const { entity_id } = z.object({ entity_id: z.string() }).parse(args);
    const response = await axios.get(`${CORE_API_URL}/api/v1/entities/${entity_id}/snapshot`);

    const summary = `⚡ Fast snapshot for "${entity_id}"\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(response.data, null, 2),
        },
      ],
    };
  }

  private async analyzeChanges(args: unknown) {
    const { entity_id, from_time, to_time } = AnalyzeChangesSchema.parse(args);

    // Get state at from_time
    const beforeResponse = await axios.get(
      `${CORE_API_URL}/api/v1/entities/${entity_id}/state`,
      { params: { as_of: from_time } }
    );

    // Get state at to_time (or current)
    const afterParams = to_time ? { as_of: to_time } : {};
    const afterResponse = await axios.get(
      `${CORE_API_URL}/api/v1/entities/${entity_id}/state`,
      { params: afterParams }
    );

    const before = beforeResponse.data.current_state || {};
    const after = afterResponse.data.current_state || {};

    // Calculate diff
    const changes = this.calculateDiff(before, after);

    const summary = `🔍 Change Analysis for "${entity_id}"\n` +
      `📅 From: ${from_time}\n` +
      `📅 To: ${to_time || 'now'}\n` +
      `➕ Added fields: ${changes.added.length}\n` +
      `✏️  Modified fields: ${changes.modified.length}\n` +
      `➖ Removed fields: ${changes.removed.length}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(changes, null, 2),
        },
      ],
    };
  }

  private calculateDiff(before: any, after: any) {
    const added: string[] = [];
    const modified: Array<{ field: string; before: any; after: any }> = [];
    const removed: string[] = [];

    // Find added and modified
    for (const key in after) {
      if (!(key in before)) {
        added.push(key);
      } else if (JSON.stringify(before[key]) !== JSON.stringify(after[key])) {
        modified.push({ field: key, before: before[key], after: after[key] });
      }
    }

    // Find removed
    for (const key in before) {
      if (!(key in after)) {
        removed.push(key);
      }
    }

    return { added, modified, removed };
  }

  private async findPatterns(args: unknown) {
    const params = FindPatternsSchema.parse(args);

    // Query events
    const response = await axios.get(`${CORE_API_URL}/api/v1/events/query`, {
      params: {
        entity_id: params.entity_id,
        event_type: params.event_type,
        since: params.since,
      },
    });

    const events = response.data.events || [];

    let analysis: any = {};

    if (params.pattern_type === 'frequency' || !params.pattern_type) {
      // Frequency analysis
      const frequencyMap: Record<string, number> = {};
      events.forEach((event: any) => {
        frequencyMap[event.event_type] = (frequencyMap[event.event_type] || 0) + 1;
      });

      analysis.frequency = Object.entries(frequencyMap)
        .map(([type, count]) => ({ event_type: type, count }))
        .sort((a, b) => b.count - a.count);
    }

    if (params.pattern_type === 'sequence' || !params.pattern_type) {
      // Sequence analysis
      const sequences: string[] = [];
      for (let i = 0; i < Math.min(events.length - 1, 10); i++) {
        sequences.push(`${events[i].event_type} → ${events[i + 1].event_type}`);
      }
      analysis.common_sequences = sequences;
    }

    const summary = `🔎 Pattern Analysis\n` +
      `📊 Events analyzed: ${events.length}\n` +
      `🎯 Pattern type: ${params.pattern_type || 'all'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(analysis, null, 2),
        },
      ],
    };
  }

  private async compareEntities(args: unknown) {
    const { entity_ids, timeframe } = CompareEntitiesSchema.parse(args);

    const comparisons = await Promise.all(
      entity_ids.map(async (id) => {
        const params = timeframe ? { since: timeframe } : {};
        const response = await axios.get(`${CORE_API_URL}/api/v1/events/query`, {
          params: { entity_id: id, ...params },
        });

        return {
          entity_id: id,
          event_count: response.data.count,
          event_types: [...new Set(response.data.events.map((e: any) => e.event_type))],
        };
      })
    );

    const summary = `🔬 Entity Comparison\n` +
      `📊 Entities compared: ${entity_ids.length}\n` +
      `⏰ Timeframe: ${timeframe || 'all time'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(comparisons, null, 2),
        },
      ],
    };
  }

  private async eventTimeline(args: unknown) {
    const { entity_id, since, until } = EventTimelineSchema.parse(args);

    const response = await axios.get(`${CORE_API_URL}/api/v1/events/query`, {
      params: { entity_id, since, until },
    });

    const events = response.data.events || [];

    const timeline = events.map((event: any, index: number) => ({
      step: index + 1,
      timestamp: event.timestamp,
      event_type: event.event_type,
      summary: `${event.event_type} - ${JSON.stringify(event.payload).slice(0, 100)}...`,
    }));

    const summary = `📅 Timeline for "${entity_id}"\n` +
      `📊 Events: ${events.length}\n` +
      `⏰ Period: ${since || 'start'} to ${until || 'now'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(timeline, null, 2),
        },
      ],
    };
  }

  private async explainEntity(args: unknown) {
    const { entity_id } = z.object({ entity_id: z.string() }).parse(args);

    // Get current state
    const stateResponse = await axios.get(
      `${CORE_API_URL}/api/v1/entities/${entity_id}/state`
    );

    // Get all events
    const eventsResponse = await axios.get(`${CORE_API_URL}/api/v1/events/query`, {
      params: { entity_id },
    });

    const state = stateResponse.data;
    const events = eventsResponse.data.events || [];

    const eventTypes = [...new Set(events.map((e: any) => e.event_type))];

    const explanation = {
      entity_id,
      current_state: state.current_state,
      total_events: events.length,
      event_types: eventTypes,
      created_at: events[0]?.timestamp,
      last_updated: state.last_updated,
      lifecycle: events.map((e: any) => ({
        when: e.timestamp,
        what: e.event_type,
      })),
    };

    const summary = `📋 Entity Explanation: "${entity_id}"\n\n` +
      `🔹 Total Events: ${events.length}\n` +
      `🔹 Event Types: ${eventTypes.length}\n` +
      `🔹 Created: ${events[0]?.timestamp || 'unknown'}\n` +
      `🔹 Last Updated: ${state.last_updated}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(explanation, null, 2),
        },
      ],
    };
  }

  private async ingestEvent(args: unknown) {
    const eventData = IngestEventSchema.parse(args);
    const response = await axios.post(`${CORE_API_URL}/api/v1/events`, eventData);

    const summary = `✅ Event ingested successfully\n` +
      `🆔 Event ID: ${response.data.event_id}\n` +
      `⏰ Timestamp: ${response.data.timestamp}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(response.data, null, 2),
        },
      ],
    };
  }

  private async getStats() {
    const response = await axios.get(`${CORE_API_URL}/api/v1/stats`);

    const summary = `📊 AllSource Statistics\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(response.data, null, 2),
        },
      ],
    };
  }

  private async getClusterStatus() {
    const response = await axios.get(`${CONTROL_PLANE_URL}/api/v1/cluster/status`);

    const summary = `🎯 Cluster Status\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(response.data, null, 2),
        },
      ],
    };
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('🌟 AllSource MCP Server running on stdio');
    console.error('🤖 AI-native temporal event store interface active');
    console.error(`📡 Core API: ${CORE_API_URL}`);
    console.error(`🎛️  Control Plane: ${CONTROL_PLANE_URL}`);
  }
}

// Start server
const server = new AllSourceMCPServer();
server.start().catch(console.error);
