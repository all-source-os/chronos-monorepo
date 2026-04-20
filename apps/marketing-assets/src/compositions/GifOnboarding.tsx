import type React from "react";
import { AbsoluteFill, useCurrentFrame, interpolate, spring, useVideoConfig } from "remotion";
import { colors, fonts } from "../components/styles";

const steps = [
  { label: "1. Sign up", detail: "curl /api/v1/onboard/start", icon: ">" },
  { label: "2. Get API key", detail: "eyJhbGciOiJIUz...kN4", icon: "#" },
  { label: "3. Ingest first event", detail: "POST /api/v1/events", icon: "+" },
  { label: "4. Query your data", detail: "GET /api/v1/events/query", icon: "?" },
];

export const GifOnboarding: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const activeStep = Math.min(Math.floor(frame / 35), steps.length - 1);

  return (
    <AbsoluteFill style={{ background: colors.bg, padding: 48, display: "flex", flexDirection: "column", justifyContent: "center", fontFamily: fonts.sans }}>
      <h2 style={{ fontSize: 28, fontWeight: 800, color: colors.text, margin: 0, marginBottom: 8 }}>
        From zero to events in 60 seconds
      </h2>
      <p style={{ fontSize: 15, color: colors.textMuted, margin: 0, marginBottom: 36 }}>
        No dashboard needed. Just your terminal.
      </p>

      <div style={{ display: "flex", flexDirection: "column", gap: 16 }}>
        {steps.map((step, i) => {
          const isActive = i === activeStep;
          const isDone = i < activeStep;
          const delay = i * 35;
          const opacity = interpolate(frame, [delay, delay + 10], [0, 1], { extrapolateLeft: "clamp", extrapolateRight: "clamp" });
          const scale = spring({ frame: frame - delay, fps, config: { damping: 15 } });

          return (
            <div
              key={step.label}
              style={{
                display: "flex",
                alignItems: "center",
                gap: 16,
                padding: "14px 20px",
                background: isActive ? `${colors.primary}15` : colors.bgCard,
                border: `1px solid ${isActive ? colors.primary : colors.border}`,
                borderRadius: 12,
                opacity,
                transform: `scale(${Math.min(scale, 1)})`,
              }}
            >
              {/* Step icon */}
              <div style={{
                width: 36,
                height: 36,
                borderRadius: 10,
                background: isDone ? colors.green : isActive ? colors.primary : colors.bgInput,
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                fontSize: 16,
                fontWeight: 700,
                color: "white",
                flexShrink: 0,
              }}>
                {isDone ? "\u2713" : step.icon}
              </div>

              <div>
                <div style={{ fontSize: 15, fontWeight: 600, color: isActive ? colors.text : isDone ? colors.textMuted : colors.textDim }}>
                  {step.label}
                </div>
                <div style={{ fontSize: 12, color: colors.textDim, fontFamily: fonts.mono, marginTop: 2 }}>
                  {step.detail}
                </div>
              </div>
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};
