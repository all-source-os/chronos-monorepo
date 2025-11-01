/**
 * Test data builders and generators for E2E tests
 */

export interface EventData {
  event_type: string;
  entity_id: string;
  tenant_id: string;
  payload: Record<string, unknown>;
  metadata?: Record<string, unknown>;
}

export interface ProjectionData {
  name: string;
  projection_type: "EntitySnapshot" | "EventCounter" | "Custom" | "TimeSeries" | "Funnel";
  status: "Created" | "Running" | "Paused" | "Failed" | "Stopped" | "Rebuilding";
  config: {
    batch_size?: number;
    checkpoint_interval?: number;
    parallel_processing?: boolean;
    filters?: Record<string, unknown>;
  };
}

export interface SchemaData {
  event_type: string;
  version: number;
  schema: Record<string, unknown>;
  compatibility_mode: "None" | "Backward" | "Forward" | "Full";
}

/**
 * Generate a test event
 */
export function createTestEvent(overrides?: Partial<EventData>): EventData {
  return {
    event_type: "TestEvent",
    entity_id: `test-entity-${Date.now()}`,
    tenant_id: "test-tenant",
    payload: {
      test: true,
      timestamp: new Date().toISOString(),
    },
    metadata: {
      source: "e2e-test",
    },
    ...overrides,
  };
}

/**
 * Generate an e-commerce event
 */
export function createEcommerceEvent(
  type: "OrderPlaced" | "PaymentProcessed" | "OrderShipped" = "OrderPlaced"
): EventData {
  const orderId = `order-${Date.now()}`;
  const userId = `user-${Math.floor(Math.random() * 1000)}`;

  const payloads: Record<string, Record<string, unknown>> = {
    OrderPlaced: {
      user_id: userId,
      items: [
        {
          product_id: `product-${Math.floor(Math.random() * 100)}`,
          quantity: Math.floor(Math.random() * 5) + 1,
          price: Math.floor(Math.random() * 1000) + 10,
        },
      ],
      total_amount: Math.floor(Math.random() * 5000) + 100,
      shipping_address: {
        street: "123 Test St",
        city: "Test City",
        state: "TS",
        zip: "12345",
      },
    },
    PaymentProcessed: {
      payment_method: "test_card",
      amount: Math.floor(Math.random() * 5000) + 100,
      status: "success",
      transaction_id: `txn-${Date.now()}`,
    },
    OrderShipped: {
      tracking_number: `TRK${Date.now()}`,
      carrier: "Test Carrier",
      estimated_delivery: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString(),
    },
  };

  return {
    event_type: type,
    entity_id: orderId,
    tenant_id: "demo-tenant-001",
    payload: payloads[type] || {},
    metadata: {
      source: "e2e-test-ecommerce",
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Generate an IoT sensor event
 */
export function createIoTEvent(): EventData {
  const sensorId = `sensor-${Math.floor(Math.random() * 5) + 1}`;

  return {
    event_type: "SensorReading",
    entity_id: sensorId,
    tenant_id: "demo-tenant-001",
    payload: {
      temperature: Math.floor(Math.random() * 40) + 10, // 10-50°C
      humidity: Math.floor(Math.random() * 60) + 20, // 20-80%
      pressure: Math.floor(Math.random() * 200) + 900, // 900-1100 hPa
      location: "test-location",
    },
    metadata: {
      source: "e2e-test-iot",
      sensor_type: "DHT22",
      timestamp: new Date().toISOString(),
    },
  };
}

/**
 * Generate a test projection
 */
export function createTestProjection(overrides?: Partial<ProjectionData>): ProjectionData {
  return {
    name: `Test Projection ${Date.now()}`,
    projection_type: "EventCounter",
    status: "Created",
    config: {
      batch_size: 100,
      checkpoint_interval: 1000,
      parallel_processing: false,
    },
    ...overrides,
  };
}

/**
 * Generate a test schema
 */
export function createTestSchema(eventType?: string): SchemaData {
  return {
    event_type: eventType || `TestEvent${Date.now()}`,
    version: 1,
    schema: {
      type: "object",
      properties: {
        test: { type: "boolean" },
        timestamp: { type: "string", format: "date-time" },
      },
      required: ["test"],
    },
    compatibility_mode: "Backward",
  };
}

/**
 * Wait for a condition with timeout
 */
export async function waitFor(
  condition: () => Promise<boolean>,
  timeout = 5000,
  interval = 100
): Promise<void> {
  const startTime = Date.now();

  while (Date.now() - startTime < timeout) {
    if (await condition()) {
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, interval));
  }

  throw new Error(`Condition not met within ${timeout}ms`);
}
