import type React from "react";
import { MockDashboard } from "../components/MockDashboard";
import { MockEventList } from "../components/MockEventList";
import { MockMetricCard } from "../components/MockMetricCard";

export const HeroScreenshot: React.FC = () => (
  <MockDashboard title="Event Explorer">
    <div style={{ display: "flex", flexDirection: "column", gap: 20, height: "100%" }}>
      {/* Metrics row */}
      <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr", gap: 16 }}>
        <MockMetricCard label="Ingestion Throughput" value="469K" unit="events/sec" />
        <MockMetricCard label="Query Latency (p99)" value="11.9" unit="us" />
        <MockMetricCard label="MCP Tools" value="43" unit="available" />
      </div>

      {/* Events table */}
      <div style={{ flex: 1 }}>
        <MockEventList count={8} />
      </div>
    </div>
  </MockDashboard>
);
