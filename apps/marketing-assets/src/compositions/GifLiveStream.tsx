import type React from "react";
import { AbsoluteFill, useCurrentFrame, interpolate } from "remotion";
import { colors, fonts } from "../components/styles";
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

export const GifLiveStream: React.FC = () => {
  const frame = useCurrentFrame();
  // New event appears every 18 frames (~0.6s)
  const visibleCount = Math.min(Math.floor(frame / 18) + 1, SAMPLE_EVENTS.length);
  const events = SAMPLE_EVENTS.slice(0, visibleCount).reverse();

  return (
    <AbsoluteFill style={{ background: colors.bg, fontFamily: fonts.sans }}>
      {/* Header */}
      <div style={{ padding: "16px 24px", borderBottom: `1px solid ${colors.border}`, display: "flex", alignItems: "center", justifyContent: "space-between" }}>
        <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
          <div style={{ width: 10, height: 10, borderRadius: 5, background: colors.green, animation: "pulse 2s infinite" }} />
          <span style={{ fontSize: 16, fontWeight: 700, color: colors.text }}>Live Event Feed</span>
        </div>
        <span style={{ fontSize: 13, color: colors.textDim, fontFamily: fonts.mono }}>{visibleCount} events</span>
      </div>

      {/* Stream */}
      <div style={{ padding: "12px 24px", display: "flex", flexDirection: "column", gap: 8 }}>
        {events.map((evt, i) => {
          const entryFrame = (SAMPLE_EVENTS.length - 1 - (SAMPLE_EVENTS.indexOf(evt))) * 18;
          const age = frame - entryFrame;
          const slideIn = interpolate(age, [0, 8], [30, 0], { extrapolateRight: "clamp" });
          const fadeIn = interpolate(age, [0, 8], [0, 1], { extrapolateRight: "clamp" });

          return (
            <div
              key={evt.id}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 12,
                padding: "10px 14px",
                background: colors.bgCard,
                border: `1px solid ${i === 0 ? colors.primary + "40" : colors.borderSubtle}`,
                borderRadius: 8,
                opacity: fadeIn,
                transform: `translateY(${slideIn}px)`,
                fontFamily: fonts.mono,
                fontSize: 13,
              }}
            >
              <span style={{ color: typeColors[evt.type] || colors.text, fontWeight: 600, minWidth: 160 }}>{evt.type}</span>
              <span style={{ color: colors.textDim }}>{evt.entity}</span>
              <span style={{ marginLeft: "auto", color: colors.textDim, fontSize: 11 }}>just now</span>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
