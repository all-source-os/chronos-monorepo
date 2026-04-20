import type React from "react";
import { colors, fonts } from "./styles";
import { SAMPLE_EVENTS } from "../data/events";

const typeColors: Record<string, string> = {
  "user.signup": "#6366f1",
  "order.placed": "#22c55e",
  "payment.settled": "#eab308",
  "agent.query": "#06b6d4",
  "projection.updated": "#3b82f6",
  "schema.registered": "#a855f7",
  "user.login": "#6366f1",
  "order.shipped": "#22c55e",
};

interface Props {
  count?: number;
  highlightIndex?: number;
}

export const MockEventList: React.FC<Props> = ({ count = 6, highlightIndex }) => {
  const events = SAMPLE_EVENTS.slice(0, count);

  return (
    <div
      style={{
        background: colors.bgCard,
        border: `1px solid ${colors.border}`,
        borderRadius: 12,
        overflow: "hidden",
        fontFamily: fonts.mono,
        fontSize: 13,
      }}
    >
      {/* Header */}
      <div
        style={{
          display: "grid",
          gridTemplateColumns: "100px 160px 100px 1fr",
          padding: "10px 16px",
          borderBottom: `1px solid ${colors.border}`,
          color: colors.textMuted,
          fontSize: 11,
          fontWeight: 600,
          textTransform: "uppercase",
          letterSpacing: 1,
          fontFamily: fonts.sans,
        }}
      >
        <span>ID</span>
        <span>Type</span>
        <span>Entity</span>
        <span>Timestamp</span>
      </div>

      {/* Rows */}
      {events.map((evt, i) => (
        <div
          key={evt.id}
          style={{
            display: "grid",
            gridTemplateColumns: "100px 160px 100px 1fr",
            padding: "10px 16px",
            borderBottom: i < events.length - 1 ? `1px solid ${colors.borderSubtle}` : "none",
            background: highlightIndex === i ? `${colors.primary}15` : "transparent",
            color: colors.text,
          }}
        >
          <span style={{ color: colors.textDim }}>{evt.id}</span>
          <span style={{ color: typeColors[evt.type] || colors.text, fontWeight: 500 }}>
            {evt.type}
          </span>
          <span style={{ color: colors.textMuted }}>{evt.entity}</span>
          <span style={{ color: colors.textDim }}>
            {evt.ts.replace("T", " ").replace("Z", "")}
          </span>
        </div>
      ))}
    </div>
  );
};
