import type React from "react";
import { AbsoluteFill } from "remotion";
import { MockTerminal } from "../components/MockTerminal";
import { colors, fonts } from "../components/styles";

export const GifApiKeyCreation: React.FC = () => (
  <AbsoluteFill style={{ background: colors.bg, padding: 40, display: "flex", flexDirection: "column", gap: 20, justifyContent: "center" }}>
    <div style={{ fontFamily: fonts.sans }}>
      <h3 style={{ fontSize: 20, fontWeight: 700, color: colors.text, margin: 0, marginBottom: 4 }}>Get your API key in one call</h3>
      <p style={{ fontSize: 14, color: colors.textMuted, margin: 0 }}>No signup form. No email verification. Just curl.</p>
    </div>
    <MockTerminal
      command={`curl -X POST https://api.all-source.xyz/api/v1/onboard/start -H "Content-Type: application/json" -d '{"email":"you@example.com","name":"My App"}'`}
      response={JSON.stringify({
        api_key: "eyJhbGciOiJIUz...kN4Wwh44",
        tenant_id: "onboard-you-at-example-xyz",
        tier: "free",
        events_quota: 100000,
      }, null, 2)}
      typingSpeed={4}
    />
  </AbsoluteFill>
);
