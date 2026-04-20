import type React from "react";
import { AbsoluteFill, Sequence, useCurrentFrame, interpolate, spring, useVideoConfig } from "remotion";
import { MockDashboard } from "../components/MockDashboard";
import { MockEventList } from "../components/MockEventList";
import { MockMetricCard } from "../components/MockMetricCard";
import { MockTerminal } from "../components/MockTerminal";
import { colors, fonts } from "../components/styles";
import { TIERS } from "../data/events";

const FadeIn: React.FC<{ children: React.ReactNode; delay?: number }> = ({ children, delay = 0 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const opacity = spring({ frame: frame - delay, fps, config: { damping: 20 } });
  const y = interpolate(opacity, [0, 1], [20, 0]);
  return (
    <div style={{ opacity, transform: `translateY(${y}px)` }}>
      {children}
    </div>
  );
};

// Scene 1: Logo + Tagline (0-5s = 0-150 frames)
const SceneLogo: React.FC = () => (
  <AbsoluteFill style={{ background: colors.bg, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", fontFamily: fonts.sans }}>
    <FadeIn>
      <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 24 }}>
        <div style={{ width: 56, height: 56, borderRadius: 16, background: `linear-gradient(135deg, ${colors.primary}, ${colors.cyan})`, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 28, fontWeight: 800, color: "white" }}>A</div>
        <span style={{ fontSize: 48, fontWeight: 800, color: colors.text, letterSpacing: -2 }}>AllSource</span>
      </div>
    </FadeIn>
    <FadeIn delay={20}>
      <p style={{ fontSize: 24, color: colors.textMuted, fontWeight: 400, margin: 0 }}>
        AI-native event store for temporal data intelligence
      </p>
    </FadeIn>
    <FadeIn delay={40}>
      <div style={{ display: "flex", gap: 32, marginTop: 40 }}>
        <Stat value="469K" label="events/sec" />
        <Stat value="11.9us" label="p99 latency" />
        <Stat value="43" label="MCP tools" />
      </div>
    </FadeIn>
  </AbsoluteFill>
);

const Stat: React.FC<{ value: string; label: string }> = ({ value, label }) => (
  <div style={{ textAlign: "center" }}>
    <div style={{ fontSize: 36, fontWeight: 800, color: colors.primary, fontFamily: fonts.mono }}>{value}</div>
    <div style={{ fontSize: 14, color: colors.textDim, marginTop: 4 }}>{label}</div>
  </div>
);

// Scene 2: Dashboard overview (5-15s = 150-450 frames)
const SceneDashboard: React.FC = () => (
  <AbsoluteFill>
    <MockDashboard title="Dashboard">
      <FadeIn>
        <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 16, marginBottom: 20 }}>
          <MockMetricCard label="Total Events" value="164,158" />
          <MockMetricCard label="Active Streams" value="12" />
          <MockMetricCard label="Projections" value="8" unit="running" />
          <MockMetricCard label="Uptime" value="99.9" unit="%" />
        </div>
      </FadeIn>
      <FadeIn delay={15}>
        <MockEventList count={5} />
      </FadeIn>
    </MockDashboard>
  </AbsoluteFill>
);

// Scene 3: Event explorer (15-30s = 450-900 frames)
const SceneExplorer: React.FC = () => {
  const frame = useCurrentFrame();
  const highlightIdx = Math.floor(frame / 30) % 8;
  return (
    <AbsoluteFill>
      <MockDashboard title="Event Explorer">
        <FadeIn>
          <MockEventList count={8} highlightIndex={highlightIdx} />
        </FadeIn>
      </MockDashboard>
    </AbsoluteFill>
  );
};

// Scene 4: Time-travel query (30-45s = 900-1350 frames)
const SceneTimeTravel: React.FC = () => (
  <AbsoluteFill style={{ background: colors.bg, padding: 60, display: "flex", flexDirection: "column", gap: 24, justifyContent: "center", fontFamily: fonts.sans }}>
    <FadeIn>
      <h2 style={{ fontSize: 32, fontWeight: 800, color: colors.text, margin: 0, marginBottom: 8 }}>Time-Travel Queries</h2>
      <p style={{ fontSize: 16, color: colors.textMuted, margin: 0 }}>Query the state of any entity at any historical timestamp</p>
    </FadeIn>
    <FadeIn delay={15}>
      <MockTerminal
        command='curl "https://api.all-source.xyz/api/v1/events/query?entity_id=ord-12c7&before=2026-04-20T09:16:00Z"'
        response={JSON.stringify({ events: [{ id: "evt-002", type: "order.placed", entity_id: "ord-12c7", timestamp: "2026-04-20T09:15:01Z", payload: { total: 149.99, items: 3 } }], count: 1 }, null, 2)}
        typingSpeed={3}
      />
    </FadeIn>
  </AbsoluteFill>
);

// Scene 5: Pricing (45-55s = 1350-1650 frames)
const ScenePricing: React.FC = () => (
  <AbsoluteFill style={{ background: colors.bg, padding: 60, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", fontFamily: fonts.sans }}>
    <FadeIn>
      <h2 style={{ fontSize: 32, fontWeight: 800, color: colors.text, margin: 0, marginBottom: 40, textAlign: "center" }}>Simple, transparent pricing</h2>
    </FadeIn>
    <div style={{ display: "grid", gridTemplateColumns: "1fr 1fr 1fr 1fr", gap: 20, width: "100%", maxWidth: 1000 }}>
      {TIERS.map((tier, i) => (
        <FadeIn key={tier.name} delay={i * 8}>
          <div style={{
            background: colors.bgCard,
            border: `1px solid ${tier.name === "Pro" ? colors.primary : colors.border}`,
            borderRadius: 16,
            padding: "28px 24px",
            textAlign: "center",
            boxShadow: tier.name === "Pro" ? `0 0 30px ${colors.primary}30` : "none",
          }}>
            <div style={{ fontSize: 14, fontWeight: 600, color: colors.textMuted, marginBottom: 8, textTransform: "uppercase", letterSpacing: 1 }}>{tier.name}</div>
            <div style={{ fontSize: 36, fontWeight: 800, color: colors.text, marginBottom: 4 }}>{tier.price}</div>
            <div style={{ fontSize: 13, color: colors.textDim }}>{tier.events}</div>
          </div>
        </FadeIn>
      ))}
    </div>
  </AbsoluteFill>
);

// Scene 6: CTA (55-60s = 1650-1800 frames)
const SceneCta: React.FC = () => (
  <AbsoluteFill style={{ background: colors.bg, display: "flex", flexDirection: "column", alignItems: "center", justifyContent: "center", fontFamily: fonts.sans }}>
    <FadeIn>
      <div style={{ display: "flex", alignItems: "center", gap: 16, marginBottom: 32 }}>
        <div style={{ width: 48, height: 48, borderRadius: 14, background: `linear-gradient(135deg, ${colors.primary}, ${colors.cyan})`, display: "flex", alignItems: "center", justifyContent: "center", fontSize: 24, fontWeight: 800, color: "white" }}>A</div>
        <span style={{ fontSize: 40, fontWeight: 800, color: colors.text, letterSpacing: -1.5 }}>AllSource</span>
      </div>
    </FadeIn>
    <FadeIn delay={15}>
      <p style={{ fontSize: 28, fontWeight: 600, color: colors.text, margin: 0, marginBottom: 12 }}>Start free. Scale to millions.</p>
    </FadeIn>
    <FadeIn delay={25}>
      <div style={{ padding: "14px 40px", borderRadius: 12, background: colors.primary, color: "white", fontSize: 18, fontWeight: 700 }}>
        all-source.xyz
      </div>
    </FadeIn>
    <FadeIn delay={35}>
      <div style={{ display: "flex", gap: 24, marginTop: 24, fontSize: 14, color: colors.textDim }}>
        <span>api.all-source.xyz</span>
        <span>status.all-source.xyz</span>
        <span>github.com/all-source-os/all-source</span>
      </div>
    </FadeIn>
  </AbsoluteFill>
);

export const DemoVideo: React.FC = () => (
  <AbsoluteFill>
    <Sequence from={0} durationInFrames={150}><SceneLogo /></Sequence>
    <Sequence from={150} durationInFrames={300}><SceneDashboard /></Sequence>
    <Sequence from={450} durationInFrames={450}><SceneExplorer /></Sequence>
    <Sequence from={900} durationInFrames={450}><SceneTimeTravel /></Sequence>
    <Sequence from={1350} durationInFrames={300}><ScenePricing /></Sequence>
    <Sequence from={1650} durationInFrames={150}><SceneCta /></Sequence>
  </AbsoluteFill>
);
