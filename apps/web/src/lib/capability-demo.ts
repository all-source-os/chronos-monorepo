export type CapabilityEvent = {
  id: string;
  version: number;
  entityId: string;
  eventType: string;
  timestamp: string;
  summary: string;
  payload: Record<string, string | number>;
};

export type OrderState = {
  orderId: string;
  customerId: string;
  status: string;
  total: string;
  payment: string;
  inventory: string;
  postcode: string;
  shipment: string;
};

export type GraphNode = {
  id: string;
  label: string;
  type: string;
  detail: string;
  x: number;
  y: number;
  visibleFrom: number;
};

export type GraphEdge = {
  source: string;
  target: string;
  label: string;
  visibleFrom: number;
};

export type McpToolName = "event_timeline" | "reconstruct_state" | "query_events";

export const ORDER_ID = "order-1042";

export const CAPABILITY_EVENTS: CapabilityEvent[] = [
  {
    id: "evt_01J5W9F2R3",
    version: 1,
    entityId: ORDER_ID,
    eventType: "order.created",
    timestamp: "2026-08-14T09:00:04Z",
    summary: "Order opened for customer cus_mara",
    payload: { customer_id: "cus_mara", total_pence: 12900, postcode: "E1 6AA" },
  },
  {
    id: "evt_01J5W9H6M8",
    version: 2,
    entityId: ORDER_ID,
    eventType: "order.confirmed",
    timestamp: "2026-08-14T09:01:18Z",
    summary: "Customer confirmed checkout",
    payload: { status: "confirmed", item_count: 2 },
  },
  {
    id: "evt_01J5W9K1Q4",
    version: 3,
    entityId: ORDER_ID,
    eventType: "payment.authorized",
    timestamp: "2026-08-14T09:02:41Z",
    summary: "£129.00 payment authorized",
    payload: { payment_id: "pay_8841", amount_pence: 12900, payment: "authorized" },
  },
  {
    id: "evt_01J5W9N7T2",
    version: 4,
    entityId: ORDER_ID,
    eventType: "inventory.reserved",
    timestamp: "2026-08-14T09:04:09Z",
    summary: "Two items reserved in warehouse LON-2",
    payload: { reservation_id: "res_220", warehouse: "LON-2", inventory: "reserved" },
  },
  {
    id: "evt_01J5W9Q3C9",
    version: 5,
    entityId: ORDER_ID,
    eventType: "delivery.address_corrected",
    timestamp: "2026-08-14T09:05:32Z",
    summary: "Delivery postcode corrected before dispatch",
    payload: { previous_postcode: "E1 6AA", postcode: "E1 6AN" },
  },
  {
    id: "evt_01J5W9T8V6",
    version: 6,
    entityId: ORDER_ID,
    eventType: "order.dispatched",
    timestamp: "2026-08-14T09:08:17Z",
    summary: "Parcel handed to Northline Express",
    payload: { status: "dispatched", shipment_id: "ship_48", carrier: "Northline Express" },
  },
];

const INITIAL_STATE: OrderState = {
  orderId: ORDER_ID,
  customerId: "—",
  status: "not created",
  total: "—",
  payment: "pending",
  inventory: "not reserved",
  postcode: "—",
  shipment: "not assigned",
};

export function reconstructOrderState(cursor: number): OrderState {
  const state = { ...INITIAL_STATE };

  for (const event of CAPABILITY_EVENTS.slice(0, cursor + 1)) {
    switch (event.eventType) {
      case "order.created":
        state.customerId = String(event.payload.customer_id);
        state.status = "created";
        state.total = "£129.00";
        state.postcode = String(event.payload.postcode);
        break;
      case "order.confirmed":
        state.status = String(event.payload.status);
        break;
      case "payment.authorized":
        state.payment = String(event.payload.payment);
        break;
      case "inventory.reserved":
        state.inventory = String(event.payload.inventory);
        break;
      case "delivery.address_corrected":
        state.postcode = String(event.payload.postcode);
        break;
      case "order.dispatched":
        state.status = String(event.payload.status);
        state.shipment = `${String(event.payload.shipment_id)} · ${String(event.payload.carrier)}`;
        break;
      default:
        break;
    }
  }

  return state;
}

const ALL_GRAPH_NODES: GraphNode[] = [
  {
    id: "cus_mara",
    label: "Mara Chen",
    type: "customer",
    detail: "Customer who placed order-1042",
    x: 105,
    y: 78,
    visibleFrom: 0,
  },
  {
    id: ORDER_ID,
    label: "Order 1042",
    type: "order",
    detail: "£129.00 · two items",
    x: 330,
    y: 140,
    visibleFrom: 0,
  },
  {
    id: "pay_8841",
    label: "Payment 8841",
    type: "payment",
    detail: "Authorized for £129.00",
    x: 590,
    y: 65,
    visibleFrom: 2,
  },
  {
    id: "res_220",
    label: "Stock LON-2",
    type: "inventory",
    detail: "Two items reserved",
    x: 585,
    y: 215,
    visibleFrom: 3,
  },
  {
    id: "ship_48",
    label: "Shipment 48",
    type: "shipment",
    detail: "Northline Express",
    x: 330,
    y: 258,
    visibleFrom: 5,
  },
];

const ALL_GRAPH_EDGES: GraphEdge[] = [
  { source: "cus_mara", target: ORDER_ID, label: "PLACED", visibleFrom: 0 },
  { source: ORDER_ID, target: "pay_8841", label: "PAID BY", visibleFrom: 2 },
  { source: ORDER_ID, target: "res_220", label: "RESERVES", visibleFrom: 3 },
  { source: ORDER_ID, target: "ship_48", label: "SHIPS AS", visibleFrom: 5 },
];

export function graphAt(cursor: number) {
  return {
    nodes: ALL_GRAPH_NODES.filter((node) => node.visibleFrom <= cursor),
    edges: ALL_GRAPH_EDGES.filter((edge) => edge.visibleFrom <= cursor),
  };
}

export function projectionAt(cursor: number) {
  const event = CAPABILITY_EVENTS[cursor] ?? CAPABILITY_EVENTS[0]!;
  const state = reconstructOrderState(cursor);

  return {
    projection: "order-summary",
    entity_id: ORDER_ID,
    kind: "entity_table",
    version: event.version,
    applied_events: cursor + 1,
    state: {
      status: state.status,
      total: state.total,
      payment: state.payment,
      inventory: state.inventory,
      delivery_postcode: state.postcode,
      shipment: state.shipment,
    },
    as_of: event.timestamp,
  };
}

export function mcpExchange(tool: McpToolName, cursor: number) {
  const event = CAPABILITY_EVENTS[cursor] ?? CAPABILITY_EVENTS[0]!;
  const visibleEvents = CAPABILITY_EVENTS.slice(0, cursor + 1);

  if (tool === "event_timeline") {
    return {
      request: { entity_id: ORDER_ID, until: event.timestamp },
      response: {
        entity_id: ORDER_ID,
        count: visibleEvents.length,
        timeline: visibleEvents.map((item) => ({
          version: item.version,
          timestamp: item.timestamp,
          event_type: item.eventType,
          summary: item.summary,
        })),
      },
    };
  }

  if (tool === "reconstruct_state") {
    return {
      request: { entity_id: ORDER_ID, as_of: event.timestamp },
      response: {
        entity_id: ORDER_ID,
        as_of: event.timestamp,
        version: event.version,
        state: reconstructOrderState(cursor),
      },
    };
  }

  return {
    request: {
      entity_id: ORDER_ID,
      as_of: event.timestamp,
      limit: 10,
      format: "json",
    },
    response: {
      count: visibleEvents.length,
      events: visibleEvents.map((item) => ({
        id: item.id,
        entity_id: item.entityId,
        event_type: item.eventType,
        timestamp: item.timestamp,
        version: item.version,
      })),
    },
  };
}

export function formatEventTime(timestamp: string) {
  return new Intl.DateTimeFormat("en-GB", {
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
    timeZone: "UTC",
  }).format(new Date(timestamp));
}
