import type { CreateEventRequest } from "./types";

// Demo tenant IDs
export const DEMO_TENANT_ID = "demo-tenant-001";

// Generate sample e-commerce events
export function generateEcommerceEvents(count: number = 50): CreateEventRequest[] {
  const events: CreateEventRequest[] = [];
  const userIds = Array.from({ length: 10 }, (_, i) => `user-${i + 1}`);
  const productIds = Array.from({ length: 20 }, (_, i) => `product-${i + 1}`);
  const orderIds = Array.from({ length: 15 }, (_, i) => `order-${i + 1}`);

  const eventTypes = [
    "UserRegistered",
    "UserLoggedIn",
    "ProductViewed",
    "ProductAddedToCart",
    "CartUpdated",
    "OrderPlaced",
    "PaymentProcessed",
    "OrderShipped",
    "OrderDelivered",
  ];

  for (let i = 0; i < count; i++) {
    const eventType = eventTypes[Math.floor(Math.random() * eventTypes.length)] || "ProductViewed";
    const userId = userIds[Math.floor(Math.random() * userIds.length)] || "user-1";
    const productId = productIds[Math.floor(Math.random() * productIds.length)] || "product-1";
    const orderId = orderIds[Math.floor(Math.random() * orderIds.length)] || "order-1";

    let payload: Record<string, unknown> = {};
    let entityId = userId;

    switch (eventType) {
      case "UserRegistered":
        payload = {
          email: `${userId}@example.com`,
          name: `User ${userId}`,
          registration_source: "web",
        };
        break;
      case "UserLoggedIn":
        payload = {
          ip_address: `192.168.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}`,
          user_agent: "Mozilla/5.0",
        };
        break;
      case "ProductViewed":
        entityId = productId;
        payload = {
          user_id: userId,
          product_name: `Product ${productId}`,
          category: ["Electronics", "Clothing", "Books", "Home"][Math.floor(Math.random() * 4)],
          price: Math.floor(Math.random() * 1000) + 10,
        };
        break;
      case "ProductAddedToCart":
        entityId = userId;
        payload = {
          product_id: productId,
          quantity: Math.floor(Math.random() * 5) + 1,
          price: Math.floor(Math.random() * 1000) + 10,
        };
        break;
      case "CartUpdated":
        entityId = userId;
        payload = {
          items_count: Math.floor(Math.random() * 10) + 1,
          total_amount: Math.floor(Math.random() * 5000) + 50,
        };
        break;
      case "OrderPlaced":
        entityId = orderId;
        payload = {
          user_id: userId,
          items: [
            {
              product_id: productId,
              quantity: Math.floor(Math.random() * 3) + 1,
              price: Math.floor(Math.random() * 1000) + 10,
            },
          ],
          total_amount: Math.floor(Math.random() * 5000) + 100,
          shipping_address: {
            street: "123 Main St",
            city: "San Francisco",
            state: "CA",
            zip: "94102",
          },
        };
        break;
      case "PaymentProcessed":
        entityId = orderId;
        payload = {
          payment_method: ["credit_card", "paypal", "bank_transfer"][Math.floor(Math.random() * 3)],
          amount: Math.floor(Math.random() * 5000) + 100,
          status: "success",
          transaction_id: `txn-${Date.now()}-${Math.random().toString(36).substring(7)}`,
        };
        break;
      case "OrderShipped":
        entityId = orderId;
        payload = {
          tracking_number:
            `TRK${Date.now()}${Math.random().toString(36).substring(7)}`.toUpperCase(),
          carrier: ["UPS", "FedEx", "USPS", "DHL"][Math.floor(Math.random() * 4)],
          estimated_delivery: new Date(Date.now() + 3 * 24 * 60 * 60 * 1000).toISOString(),
        };
        break;
      case "OrderDelivered":
        entityId = orderId;
        payload = {
          delivered_at: new Date().toISOString(),
          signature_required: Math.random() > 0.5,
        };
        break;
    }

    events.push({
      event_type: eventType,
      entity_id: entityId,
      tenant_id: DEMO_TENANT_ID,
      payload,
      metadata: {
        source: "demo",
        environment: "development",
        timestamp: new Date(Date.now() - Math.random() * 7 * 24 * 60 * 60 * 1000).toISOString(),
      },
    });
  }

  return events;
}

// Generate IoT sensor events
export function generateIoTEvents(count: number = 100): CreateEventRequest[] {
  const events: CreateEventRequest[] = [];
  const sensorIds = Array.from({ length: 5 }, (_, i) => `sensor-${i + 1}`);
  const locations = ["warehouse-1", "warehouse-2", "factory-floor", "loading-dock", "storage-room"];

  for (let i = 0; i < count; i++) {
    const sensorId = sensorIds[Math.floor(Math.random() * sensorIds.length)] || "sensor-1";
    const location = locations[Math.floor(Math.random() * locations.length)] || "warehouse-1";

    events.push({
      event_type: "SensorReading",
      entity_id: sensorId,
      tenant_id: DEMO_TENANT_ID,
      payload: {
        temperature: Math.floor(Math.random() * 40) + 10, // 10-50°C
        humidity: Math.floor(Math.random() * 60) + 20, // 20-80%
        pressure: Math.floor(Math.random() * 200) + 900, // 900-1100 hPa
        location,
      },
      metadata: {
        source: "iot-gateway",
        sensor_type: "DHT22",
        timestamp: new Date(Date.now() - Math.random() * 24 * 60 * 60 * 1000).toISOString(),
      },
    });
  }

  return events;
}

// Generate user activity events
export function generateUserActivityEvents(count: number = 75): CreateEventRequest[] {
  const events: CreateEventRequest[] = [];
  const userIds = Array.from({ length: 20 }, (_, i) => `user-${i + 1}`);
  const pages = ["/home", "/products", "/cart", "/checkout", "/profile", "/orders", "/help"];
  const actions = [
    "PageViewed",
    "ButtonClicked",
    "FormSubmitted",
    "SearchPerformed",
    "FilterApplied",
  ];

  for (let i = 0; i < count; i++) {
    const userId = userIds[Math.floor(Math.random() * userIds.length)] || "user-1";
    const action = actions[Math.floor(Math.random() * actions.length)] || "PageViewed";
    const page = pages[Math.floor(Math.random() * pages.length)] || "/home";

    events.push({
      event_type: action,
      entity_id: userId,
      tenant_id: DEMO_TENANT_ID,
      payload: {
        page,
        session_id: `session-${Math.random().toString(36).substring(7)}`,
        duration_ms: Math.floor(Math.random() * 30000),
        referrer: Math.random() > 0.5 ? "google.com" : "direct",
      },
      metadata: {
        source: "web-analytics",
        user_agent: "Mozilla/5.0",
        timestamp: new Date(Date.now() - Math.random() * 12 * 60 * 60 * 1000).toISOString(),
      },
    });
  }

  return events;
}

// Sample schemas
export const sampleSchemas = [
  {
    event_type: "UserRegistered",
    version: 1,
    schema: {
      type: "object",
      properties: {
        email: { type: "string", format: "email" },
        name: { type: "string", minLength: 1 },
        registration_source: { type: "string", enum: ["web", "mobile", "api"] },
      },
      required: ["email", "name"],
    },
    compatibility_mode: "Backward" as const,
  },
  {
    event_type: "OrderPlaced",
    version: 1,
    schema: {
      type: "object",
      properties: {
        user_id: { type: "string" },
        items: {
          type: "array",
          items: {
            type: "object",
            properties: {
              product_id: { type: "string" },
              quantity: { type: "number", minimum: 1 },
              price: { type: "number", minimum: 0 },
            },
            required: ["product_id", "quantity", "price"],
          },
        },
        total_amount: { type: "number", minimum: 0 },
        shipping_address: { type: "object" },
      },
      required: ["user_id", "items", "total_amount"],
    },
    compatibility_mode: "Full" as const,
  },
  {
    event_type: "SensorReading",
    version: 1,
    schema: {
      type: "object",
      properties: {
        temperature: { type: "number" },
        humidity: { type: "number", minimum: 0, maximum: 100 },
        pressure: { type: "number" },
        location: { type: "string" },
      },
      required: ["temperature", "humidity"],
    },
    compatibility_mode: "Forward" as const,
  },
];
