import type React from "react";
import { useCurrentFrame, interpolate } from "remotion";
import { colors, fonts } from "./styles";

interface Props {
  command: string;
  response: string;
  typingSpeed?: number; // chars per frame
}

export const MockTerminal: React.FC<Props> = ({
  command,
  response,
  typingSpeed = 2,
}) => {
  const frame = useCurrentFrame();
  const cmdChars = Math.min(Math.floor(frame * typingSpeed), command.length);
  const cmdDone = cmdChars >= command.length;
  const responseDelay = Math.ceil(command.length / typingSpeed) + 10;
  const responseOpacity = interpolate(
    frame,
    [responseDelay, responseDelay + 8],
    [0, 1],
    { extrapolateLeft: "clamp", extrapolateRight: "clamp" }
  );

  return (
    <div
      style={{
        background: "#0c0c14",
        border: `1px solid ${colors.border}`,
        borderRadius: 12,
        overflow: "hidden",
        fontFamily: fonts.mono,
        fontSize: 13,
        lineHeight: 1.6,
      }}
    >
      {/* Title bar */}
      <div
        style={{
          display: "flex",
          alignItems: "center",
          gap: 8,
          padding: "10px 16px",
          background: "#08080e",
          borderBottom: `1px solid ${colors.border}`,
        }}
      >
        <div style={{ width: 12, height: 12, borderRadius: 6, background: "#ef4444" }} />
        <div style={{ width: 12, height: 12, borderRadius: 6, background: "#eab308" }} />
        <div style={{ width: 12, height: 12, borderRadius: 6, background: "#22c55e" }} />
        <span style={{ color: colors.textDim, fontSize: 12, marginLeft: 8, fontFamily: fonts.sans }}>
          Terminal
        </span>
      </div>

      {/* Content */}
      <div style={{ padding: "16px 20px" }}>
        {/* Command line */}
        <div style={{ display: "flex", gap: 8 }}>
          <span style={{ color: colors.green }}>$</span>
          <span style={{ color: colors.text }}>
            {command.slice(0, cmdChars)}
            {!cmdDone && (
              <span
                style={{
                  display: "inline-block",
                  width: 8,
                  height: 16,
                  background: colors.text,
                  marginLeft: 1,
                  verticalAlign: "text-bottom",
                  opacity: frame % 30 < 15 ? 1 : 0,
                }}
              />
            )}
          </span>
        </div>

        {/* Response */}
        {cmdDone && (
          <pre
            style={{
              color: colors.textMuted,
              marginTop: 12,
              opacity: responseOpacity,
              whiteSpace: "pre-wrap",
              margin: 0,
              marginTop: 12,
            }}
          >
            {response}
          </pre>
        )}
      </div>
    </div>
  );
};
