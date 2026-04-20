import type React from "react";
import { colors, fonts } from "./styles";

const sidebarItems = [
  { icon: "~", label: "Dashboard", active: false },
  { icon: ">", label: "Events", active: true },
  { icon: "#", label: "Projections", active: false },
  { icon: "*", label: "Schemas", active: false },
  { icon: "!", label: "Pipelines", active: false },
  { icon: "@", label: "Settings", active: false },
];

interface Props {
  children: React.ReactNode;
  title?: string;
}

export const MockDashboard: React.FC<Props> = ({ children, title = "Event Explorer" }) => (
  <div
    style={{
      display: "flex",
      width: "100%",
      height: "100%",
      background: colors.bg,
      fontFamily: fonts.sans,
      color: colors.text,
    }}
  >
    {/* Sidebar */}
    <div
      style={{
        width: 220,
        background: colors.bgSidebar,
        borderRight: `1px solid ${colors.border}`,
        display: "flex",
        flexDirection: "column",
        padding: "20px 0",
        flexShrink: 0,
      }}
    >
      {/* Logo */}
      <div
        style={{
          padding: "0 20px 24px",
          display: "flex",
          alignItems: "center",
          gap: 10,
          borderBottom: `1px solid ${colors.border}`,
          marginBottom: 16,
        }}
      >
        <div
          style={{
            width: 28,
            height: 28,
            borderRadius: 8,
            background: `linear-gradient(135deg, ${colors.primary}, ${colors.cyan})`,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            fontSize: 14,
            fontWeight: 800,
            color: "white",
          }}
        >
          A
        </div>
        <span style={{ fontSize: 16, fontWeight: 700, letterSpacing: -0.3 }}>AllSource</span>
      </div>

      {/* Nav items */}
      {sidebarItems.map((item) => (
        <div
          key={item.label}
          style={{
            padding: "10px 20px",
            display: "flex",
            alignItems: "center",
            gap: 12,
            fontSize: 14,
            fontWeight: item.active ? 600 : 400,
            color: item.active ? colors.text : colors.textMuted,
            background: item.active ? `${colors.primary}15` : "transparent",
            borderLeft: item.active ? `2px solid ${colors.primary}` : "2px solid transparent",
            cursor: "pointer",
          }}
        >
          <span style={{ fontFamily: fonts.mono, fontSize: 14, width: 16 }}>{item.icon}</span>
          {item.label}
        </div>
      ))}

      {/* Bottom status */}
      <div style={{ marginTop: "auto", padding: "16px 20px", borderTop: `1px solid ${colors.border}` }}>
        <div style={{ display: "flex", alignItems: "center", gap: 8, fontSize: 12, color: colors.textDim }}>
          <div style={{ width: 8, height: 8, borderRadius: 4, background: colors.green }} />
          All systems healthy
        </div>
        <div style={{ fontSize: 11, color: colors.textDim, marginTop: 4 }}>v0.19.1 | Pro tier</div>
      </div>
    </div>

    {/* Main content */}
    <div style={{ flex: 1, display: "flex", flexDirection: "column", overflow: "hidden" }}>
      {/* Header */}
      <div
        style={{
          padding: "16px 28px",
          borderBottom: `1px solid ${colors.border}`,
          display: "flex",
          alignItems: "center",
          justifyContent: "space-between",
        }}
      >
        <h1 style={{ fontSize: 20, fontWeight: 700, margin: 0 }}>{title}</h1>
        <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
          <div
            style={{
              padding: "6px 14px",
              borderRadius: 8,
              background: colors.bgInput,
              border: `1px solid ${colors.border}`,
              color: colors.textDim,
              fontSize: 13,
              width: 200,
            }}
          >
            Search events...
          </div>
          <div
            style={{
              width: 32,
              height: 32,
              borderRadius: 16,
              background: `linear-gradient(135deg, ${colors.primary}, ${colors.cyan})`,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              fontSize: 13,
              fontWeight: 700,
              color: "white",
            }}
          >
            D
          </div>
        </div>
      </div>

      {/* Content */}
      <div style={{ flex: 1, padding: 28, overflow: "hidden" }}>{children}</div>
    </div>
  </div>
);
