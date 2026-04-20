import type React from "react";
import { AbsoluteFill, useCurrentFrame, interpolate } from "remotion";
import { MockDashboard } from "../components/MockDashboard";
import { MockEventList } from "../components/MockEventList";
import { colors, fonts } from "../components/styles";

export const GifEventExplorer: React.FC = () => {
  const frame = useCurrentFrame();
  const highlightIdx = Math.floor(frame / 25) % 8;
  const searchProgress = interpolate(frame, [0, 60], [0, 1], { extrapolateRight: "clamp" });
  const searchText = "order.placed".slice(0, Math.floor(searchProgress * 12));

  return (
    <AbsoluteFill>
      <MockDashboard title="Event Explorer">
        <div style={{ display: "flex", flexDirection: "column", gap: 12, height: "100%" }}>
          {/* Search bar with typing animation */}
          <div style={{
            padding: "8px 14px",
            borderRadius: 8,
            background: colors.bgInput,
            border: `1px solid ${colors.border}`,
            fontFamily: fonts.mono,
            fontSize: 13,
            color: searchText ? colors.text : colors.textDim,
          }}>
            {searchText || "Search events..."}
            {frame < 60 && (
              <span style={{ opacity: frame % 20 < 10 ? 1 : 0, color: colors.text }}>|</span>
            )}
          </div>
          <MockEventList count={6} highlightIndex={highlightIdx} />
        </div>
      </MockDashboard>
    </AbsoluteFill>
  );
};
