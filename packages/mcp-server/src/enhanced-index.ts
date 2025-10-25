#!/usr/bin/env node

/**
 * Enhanced AllSource MCP Server - v2.0
 *
 * Exposes all AllSource capabilities:
 * - Advanced Query DSL (Clojure)
 * - Projection Management
 * - Event Processing Pipelines
 * - Analytics Engine (time-series, funnels, anomalies)
 * - Integration Tools (replay, validation, migration)
 * - Policy & Governance (Go control plane)
 */

import { Server } from '@modelcontextprotocol/sdk/server/index.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from '@modelcontextprotocol/sdk/types.js';
import axios, { AxiosInstance } from 'axios';
import { z } from 'zod';

// Service URLs
const RUST_CORE_URL = process.env.ALLSOURCE_CORE_URL || 'http://localhost:8080';
const GO_CONTROL_URL = process.env.ALLSOURCE_CONTROL_URL || 'http://localhost:8081';
const CLOJURE_QUERY_URL = process.env.ALLSOURCE_CLOJURE_URL || 'http://localhost:7888';

// API Keys for authentication
const CLOJURE_API_KEY = process.env.ALLSOURCE_CLOJURE_API_KEY;
const CONTROL_API_KEY = process.env.ALLSOURCE_CONTROL_API_KEY;

// ============================================================================
// Service Clients
// ============================================================================

class ClojureQueryClient {
  private client: AxiosInstance;

  constructor(baseURL: string, apiKey?: string) {
    this.client = axios.create({
      baseURL,
      headers: apiKey ? { 'Authorization': `Bearer ${apiKey}` } : {},
      timeout: 30000,
    });
  }

  async executeQuery(query: any) {
    const response = await this.client.post('/api/v1/query/execute', { query });
    return response.data;
  }

  async createProjection(projection: any) {
    const response = await this.client.post('/api/v1/projections', projection);
    return response.data;
  }

  async getProjectionState(name: string, entityId: string) {
    const response = await this.client.get(`/api/v1/projections/${name}/state/${entityId}`);
    return response.data;
  }

  async listProjections() {
    const response = await this.client.get('/api/v1/projections');
    return response.data;
  }

  async executePipeline(pipeline: any, events: any[]) {
    const response = await this.client.post('/api/v1/pipelines/execute', {
      pipeline,
      events,
    });
    return response.data;
  }

  async computeTimeSeries(config: any) {
    const response = await this.client.post('/api/v1/analytics/time-series', config);
    return response.data;
  }

  async analyzeFunnel(config: any) {
    const response = await this.client.post('/api/v1/analytics/funnel', config);
    return response.data;
  }

  async detectAnomalies(config: any) {
    const response = await this.client.post('/api/v1/analytics/anomalies', config);
    return response.data;
  }

  async replayEvents(config: any) {
    const response = await this.client.post('/api/v1/integration/replay', config);
    return response.data;
  }

  async validateEvents(config: any, events: any[]) {
    const response = await this.client.post('/api/v1/integration/validate', {
      config,
      events,
    });
    return response.data;
  }
}

class PolicyEngineClient {
  private client: AxiosInstance;

  constructor(baseURL: string, apiKey?: string) {
    this.client = axios.create({
      baseURL,
      headers: apiKey ? { 'Authorization': `Bearer ${apiKey}` } : {},
      timeout: 10000,
    });
  }

  async createPolicy(policy: any) {
    const response = await this.client.post('/api/v1/policies', policy);
    return response.data;
  }

  async evaluatePolicy(request: any) {
    const response = await this.client.post('/api/v1/policies/evaluate', request);
    return response.data;
  }

  async listPolicies(tenantId?: string) {
    const params = tenantId ? { tenant_id: tenantId } : {};
    const response = await this.client.get('/api/v1/policies', { params });
    return response.data;
  }
}

// ============================================================================
// Zod Schemas for Enhanced Tools
// ============================================================================

const AdvancedQuerySchema = z.object({
  select: z.array(z.string()).optional(),
  from: z.string(),
  where: z.any().optional(),
  aggregations: z.array(z.object({
    function: z.enum(['count', 'sum', 'avg', 'min', 'max', 'p50', 'p95', 'p99', 'stddev', 'distinct']),
    field: z.union([z.string(), z.array(z.string())]).optional(),
    alias: z.string(),
  })).optional(),
  groupBy: z.array(z.string()).optional(),
  orderBy: z.array(z.tuple([z.string(), z.enum(['asc', 'desc'])])).optional(),
  limit: z.number().optional(),
});

const TimeSeriesSchema = z.object({
  event_type: z.string().optional(),
  interval: z.enum(['second', 'minute', 'hour', 'day', 'week', 'month']),
  aggregations: z.array(z.object({
    function: z.string(),
    field: z.string().optional(),
    alias: z.string(),
  })),
  start_time: z.string(),
  end_time: z.string(),
  fill_missing: z.enum(['zero', 'null', 'forward_fill']).optional(),
});

const FunnelAnalysisSchema = z.object({
  funnel_name: z.string(),
  steps: z.array(z.object({
    name: z.string(),
    event_type: z.string(),
    order: z.number(),
  })),
  time_window_ms: z.number().optional(),
  since: z.string().optional(),
});

const AnomalyDetectionSchema = z.object({
  metric_name: z.string(),
  metric_field: z.string(),
  algorithm: z.enum(['zscore', 'iqr', 'mad']).optional(),
  sensitivity: z.number().optional(),
  baseline_window: z.number().optional(),
  since: z.string(),
});

const ProjectionSchema = z.object({
  name: z.string(),
  version: z.number(),
  initial_state: z.record(z.any()),
  projection_logic: z.string(),
  auto_start: z.boolean().optional(),
});

const PipelineSchema = z.object({
  name: z.string(),
  version: z.number(),
  operators: z.array(z.object({
    type: z.enum(['filter', 'map', 'enrich', 'window', 'batch', 'aggregate']),
    name: z.string(),
    config: z.record(z.any()).optional(),
  })),
  backpressure: z.object({
    strategy: z.enum(['drop', 'buffer', 'block']),
    buffer_size: z.number().optional(),
  }).optional(),
});

const PolicySchema = z.object({
  name: z.string(),
  tenant_id: z.string(),
  rules: z.array(z.object({
    effect: z.enum(['allow', 'deny']),
    actions: z.array(z.string()),
    resources: z.array(z.string()),
    conditions: z.record(z.any()).optional(),
  })),
});

// ============================================================================
// Enhanced MCP Tools Definition
// ============================================================================

const enhancedTools: Tool[] = [
  // ========== Advanced Query & Analytics ==========
  {
    name: 'advanced_query',
    description: 'Execute advanced queries using Clojure Query DSL with aggregations, grouping, and complex filters. Perfect for business intelligence and reporting.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'object',
          description: 'Query DSL object',
          properties: {
            select: { type: 'array', items: { type: 'string' } },
            from: { type: 'string' },
            where: { type: 'array' },
            aggregations: { type: 'array' },
            groupBy: { type: 'array' },
            orderBy: { type: 'array' },
            limit: { type: 'number' },
          },
          required: ['from'],
        },
      },
      required: ['query'],
    },
  },
  {
    name: 'time_series_analysis',
    description: 'Analyze events over time with configurable intervals (second/minute/hour/day/week/month) and multiple aggregations (count/sum/avg/percentiles).',
    inputSchema: {
      type: 'object',
      properties: {
        event_type: { type: 'string' },
        interval: { type: 'string', enum: ['second', 'minute', 'hour', 'day', 'week', 'month'] },
        aggregations: { type: 'array' },
        start_time: { type: 'string' },
        end_time: { type: 'string' },
        fill_missing: { type: 'string', enum: ['zero', 'null', 'forward_fill'] },
      },
      required: ['interval', 'aggregations', 'start_time', 'end_time'],
    },
  },
  {
    name: 'funnel_analysis',
    description: 'Analyze conversion funnels to understand user drop-off at each step. Essential for product analytics and optimization.',
    inputSchema: {
      type: 'object',
      properties: {
        funnel_name: { type: 'string' },
        steps: { type: 'array' },
        time_window_ms: { type: 'number' },
        since: { type: 'string' },
      },
      required: ['funnel_name', 'steps'],
    },
  },
  {
    name: 'detect_anomalies',
    description: 'Detect anomalies in time-series data using statistical methods (Z-score, IQR, MAD). Useful for alerting and monitoring.',
    inputSchema: {
      type: 'object',
      properties: {
        metric_name: { type: 'string' },
        metric_field: { type: 'string' },
        algorithm: { type: 'string', enum: ['zscore', 'iqr', 'mad'] },
        sensitivity: { type: 'number' },
        baseline_window: { type: 'number' },
        since: { type: 'string' },
      },
      required: ['metric_name', 'metric_field', 'since'],
    },
  },

  // ========== Projection Management ==========
  {
    name: 'create_projection',
    description: 'Create a new projection that maintains a queryable read model from event streams. Projections update in real-time as events arrive.',
    inputSchema: {
      type: 'object',
      properties: {
        name: { type: 'string' },
        version: { type: 'number' },
        initial_state: { type: 'object' },
        projection_logic: { type: 'string', description: 'Clojure function as string' },
        auto_start: { type: 'boolean' },
      },
      required: ['name', 'version', 'initial_state', 'projection_logic'],
    },
  },
  {
    name: 'get_projection_state',
    description: 'Query the current state of a projection for a specific entity. Much faster than event replay.',
    inputSchema: {
      type: 'object',
      properties: {
        projection_name: { type: 'string' },
        entity_id: { type: 'string' },
      },
      required: ['projection_name', 'entity_id'],
    },
  },
  {
    name: 'list_projections',
    description: 'List all available projections with their status and metrics.',
    inputSchema: {
      type: 'object',
      properties: {},
    },
  },

  // ========== Pipeline Management ==========
  {
    name: 'execute_pipeline',
    description: 'Execute an event processing pipeline with operators (filter/map/enrich/window/batch). Supports backpressure and parallel execution.',
    inputSchema: {
      type: 'object',
      properties: {
        pipeline: { type: 'object' },
        event_query: { type: 'object', description: 'Query to fetch events to process' },
      },
      required: ['pipeline', 'event_query'],
    },
  },

  // ========== Integration Tools ==========
  {
    name: 'replay_events',
    description: 'Replay historical events to rebuild projections or test pipelines. Supports speed control and filtering.',
    inputSchema: {
      type: 'object',
      properties: {
        start_time: { type: 'string' },
        end_time: { type: 'string' },
        target: { type: 'string', description: 'Projection name or pipeline ID' },
        speed: { type: 'number', description: '0 = max speed, 1 = real-time, 2 = 2x' },
        filter: { type: 'object' },
      },
      required: ['start_time', 'end_time', 'target'],
    },
  },
  {
    name: 'validate_events',
    description: 'Validate events against schema rules. Returns detailed validation errors and warnings.',
    inputSchema: {
      type: 'object',
      properties: {
        validation_rules: { type: 'array' },
        event_query: { type: 'object' },
      },
      required: ['validation_rules', 'event_query'],
    },
  },

  // ========== Policy & Governance ==========
  {
    name: 'create_policy',
    description: 'Create an access control policy with rules, conditions, and effects (allow/deny).',
    inputSchema: {
      type: 'object',
      properties: {
        name: { type: 'string' },
        tenant_id: { type: 'string' },
        rules: { type: 'array' },
      },
      required: ['name', 'tenant_id', 'rules'],
    },
  },
  {
    name: 'evaluate_policy',
    description: 'Evaluate a policy decision for a specific request. Returns allow/deny with detailed reasoning.',
    inputSchema: {
      type: 'object',
      properties: {
        tenant_id: { type: 'string' },
        action: { type: 'string' },
        resource: { type: 'string' },
        context: { type: 'object' },
      },
      required: ['tenant_id', 'action', 'resource'],
    },
  },
  {
    name: 'list_policies',
    description: 'List all policies, optionally filtered by tenant.',
    inputSchema: {
      type: 'object',
      properties: {
        tenant_id: { type: 'string' },
      },
    },
  },
];

// ============================================================================
// Enhanced MCP Server Implementation
// ============================================================================

class EnhancedAllSourceMCPServer {
  private server: Server;
  private clojureClient: ClojureQueryClient;
  private policyClient: PolicyEngineClient;
  private rustClient: AxiosInstance;

  constructor() {
    this.server = new Server(
      {
        name: 'allsource-mcp-enhanced',
        version: '2.0.0',
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.clojureClient = new ClojureQueryClient(CLOJURE_QUERY_URL, CLOJURE_API_KEY);
    this.policyClient = new PolicyEngineClient(GO_CONTROL_URL, CONTROL_API_KEY);
    this.rustClient = axios.create({
      baseURL: RUST_CORE_URL,
      timeout: 30000,
    });

    this.setupHandlers();
  }

  private setupHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: enhancedTools,
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          // Advanced Query & Analytics
          case 'advanced_query':
            return await this.advancedQuery(args);
          case 'time_series_analysis':
            return await this.timeSeriesAnalysis(args);
          case 'funnel_analysis':
            return await this.funnelAnalysis(args);
          case 'detect_anomalies':
            return await this.detectAnomalies(args);

          // Projection Management
          case 'create_projection':
            return await this.createProjection(args);
          case 'get_projection_state':
            return await this.getProjectionState(args);
          case 'list_projections':
            return await this.listProjections();

          // Pipeline Management
          case 'execute_pipeline':
            return await this.executePipeline(args);

          // Integration Tools
          case 'replay_events':
            return await this.replayEvents(args);
          case 'validate_events':
            return await this.validateEvents(args);

          // Policy & Governance
          case 'create_policy':
            return await this.createPolicy(args);
          case 'evaluate_policy':
            return await this.evaluatePolicy(args);
          case 'list_policies':
            return await this.listPolicies(args);

          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        return {
          content: [
            {
              type: 'text',
              text: `❌ Error: ${errorMessage}\n\n${error instanceof Error ? error.stack : ''}`,
            },
          ],
        };
      }
    });
  }

  // ========== Tool Implementations ==========

  private async advancedQuery(args: unknown) {
    const { query } = z.object({ query: AdvancedQuerySchema }).parse(args);
    const result = await this.clojureClient.executeQuery(query);

    const summary = `📊 Advanced Query Results\n` +
      `🔍 Query: ${query.from}\n` +
      `📈 Results: ${result.count} rows\n` +
      `⏱️  Duration: ${result.duration_ms}ms\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async timeSeriesAnalysis(args: unknown) {
    const config = TimeSeriesSchema.parse(args);
    const result = await this.clojureClient.computeTimeSeries(config);

    const summary = `📈 Time Series Analysis\n` +
      `⏰ Interval: ${config.interval}\n` +
      `📊 Data Points: ${result.points?.length || 0}\n` +
      `📅 Period: ${config.start_time} to ${config.end_time}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async funnelAnalysis(args: unknown) {
    const config = FunnelAnalysisSchema.parse(args);
    const result = await this.clojureClient.analyzeFunnel(config);

    const summary = `🎯 Funnel Analysis: ${config.funnel_name}\n` +
      `📊 Steps: ${config.steps.length}\n` +
      `📈 Conversion Rate: ${(result.conversion_rate * 100).toFixed(2)}%\n` +
      `⏱️  Avg Time: ${result.average_time}ms\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async detectAnomalies(args: unknown) {
    const config = AnomalyDetectionSchema.parse(args);
    const result = await this.clojureClient.detectAnomalies(config);

    const summary = `🚨 Anomaly Detection: ${config.metric_name}\n` +
      `🔍 Algorithm: ${config.algorithm || 'zscore'}\n` +
      `📊 Anomalies Found: ${result.anomalies?.length || 0}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async createProjection(args: unknown) {
    const projection = ProjectionSchema.parse(args);
    const result = await this.clojureClient.createProjection(projection);

    const summary = `✅ Projection Created: ${projection.name}\n` +
      `📌 Version: ${projection.version}\n` +
      `🚀 Auto-start: ${projection.auto_start ? 'yes' : 'no'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async getProjectionState(args: unknown) {
    const { projection_name, entity_id } = z.object({
      projection_name: z.string(),
      entity_id: z.string(),
    }).parse(args);

    const state = await this.clojureClient.getProjectionState(projection_name, entity_id);

    const summary = `📊 Projection State: ${projection_name}\n` +
      `🆔 Entity: ${entity_id}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(state, null, 2),
        },
      ],
    };
  }

  private async listProjections() {
    const projections = await this.clojureClient.listProjections();

    const summary = `📋 Available Projections: ${projections.length}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(projections, null, 2),
        },
      ],
    };
  }

  private async executePipeline(args: unknown) {
    const { pipeline, event_query } = z.object({
      pipeline: PipelineSchema,
      event_query: z.object({}).passthrough(),
    }).parse(args);

    // First, fetch events using query
    const eventsResponse = await this.rustClient.get('/api/v1/events/query', {
      params: event_query,
    });

    // Execute pipeline on events
    const result = await this.clojureClient.executePipeline(
      pipeline,
      eventsResponse.data.events || []
    );

    const summary = `⚙️  Pipeline Executed: ${pipeline.name}\n` +
      `📊 Input Events: ${eventsResponse.data.count}\n` +
      `📈 Output Events: ${result.result?.length || 0}\n` +
      `⏱️  Duration: ${result.duration_ms}ms\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async replayEvents(args: unknown) {
    const config = z.object({
      start_time: z.string(),
      end_time: z.string(),
      target: z.string(),
      speed: z.number().optional(),
      filter: z.object({}).passthrough().optional(),
    }).parse(args);

    const result = await this.clojureClient.replayEvents(config);

    const summary = `🔄 Event Replay Completed\n` +
      `🎯 Target: ${config.target}\n` +
      `📊 Events Replayed: ${result.events_replayed}\n` +
      `⏱️  Duration: ${result.duration_ms}ms\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async validateEvents(args: unknown) {
    const { validation_rules, event_query } = z.object({
      validation_rules: z.array(z.any()),
      event_query: z.object({}).passthrough(),
    }).parse(args);

    // Fetch events
    const eventsResponse = await this.rustClient.get('/api/v1/events/query', {
      params: event_query,
    });

    // Validate events
    const result = await this.clojureClient.validateEvents(
      { rules: validation_rules },
      eventsResponse.data.events || []
    );

    const summary = `✅ Validation Complete\n` +
      `📊 Total Events: ${result.total_events}\n` +
      `✓ Valid: ${result.valid_events}\n` +
      `✗ Invalid: ${result.invalid_events}\n` +
      `⚠️  Warnings: ${result.warnings?.length || 0}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async createPolicy(args: unknown) {
    const policy = PolicySchema.parse(args);
    const result = await this.policyClient.createPolicy(policy);

    const summary = `🔒 Policy Created: ${policy.name}\n` +
      `🏢 Tenant: ${policy.tenant_id}\n` +
      `📋 Rules: ${policy.rules.length}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(result, null, 2),
        },
      ],
    };
  }

  private async evaluatePolicy(args: unknown) {
    const request = z.object({
      tenant_id: z.string(),
      action: z.string(),
      resource: z.string(),
      context: z.object({}).passthrough().optional(),
    }).parse(args);

    const decision = await this.policyClient.evaluatePolicy(request);

    const summary = `🔐 Policy Evaluation\n` +
      `🎯 Action: ${request.action}\n` +
      `📦 Resource: ${request.resource}\n` +
      `✅ Decision: ${decision.decision}\n` +
      `📝 Reason: ${decision.reason || 'N/A'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(decision, null, 2),
        },
      ],
    };
  }

  private async listPolicies(args: unknown = {}) {
    const { tenant_id } = z.object({
      tenant_id: z.string().optional(),
    }).parse(args);

    const policies = await this.policyClient.listPolicies(tenant_id);

    const summary = `📋 Policies: ${policies.length}\n` +
      `🏢 Tenant: ${tenant_id || 'all'}\n\n`;

    return {
      content: [
        {
          type: 'text',
          text: summary + JSON.stringify(policies, null, 2),
        },
      ],
    };
  }

  async start() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('🌟 Enhanced AllSource MCP Server v2.0 running');
    console.error('🚀 Advanced analytics, projections, pipelines, and policies enabled');
    console.error(`📡 Rust Core: ${RUST_CORE_URL}`);
    console.error(`🎛️  Go Control: ${GO_CONTROL_URL}`);
    console.error(`⚡ Clojure Query: ${CLOJURE_QUERY_URL}`);
  }
}

// Start enhanced server
const server = new EnhancedAllSourceMCPServer();
server.start().catch(console.error);
