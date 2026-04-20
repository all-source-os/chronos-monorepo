import type React from "react";
import { colors, fonts } from "./styles";

interface Props {
  label: string;
  value: string;
  unit?: string;
}

export const MockMetricCard: React.FC<Props> = ({ label, value, unit }) => (
  <div
    style={{
      background: colors.bgCard,
      border: `1px solid ${colors.border}`,
      borderRadius: 12,
      padding: "20px 24px",
      display: "flex",
      flexDirection: "column",
      gap: 4,
      fontFamily: fonts.sans,
    }}
  >
    <span style={{ color: colors.textMuted, fontSize: 13, fontWeight: 500 }}>
      {label}
    </span>
    <div style={{ display: "flex", alignItems: "baseline", gap: 6 }}>
      <span style={{ color: colors.text, fontSize: 32, fontWeight: 700, letterSpacing: -1 }}>
        {value}
      </span>
      {unit && (
        <span style={{ color: colors.textDim, fontSize: 14, fontWeight: 500 }}>
          {unit}
        </span>
      )}
    </div>
  </div>
);
