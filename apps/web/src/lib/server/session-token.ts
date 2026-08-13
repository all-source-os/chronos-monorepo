export function decodeJwtPayload(token: string): Record<string, unknown> {
  try {
    const parts = token.split(".");
    if (parts.length !== 3) return {};
    const payload = Buffer.from(parts[1] ?? "", "base64url").toString("utf-8");
    return JSON.parse(payload) as Record<string, unknown>;
  } catch {
    return {};
  }
}

/**
 * Human sessions minted before the secret-boundary migration embedded a
 * long-lived Core API key. Reject them so a fresh sign-in replaces the cookie
 * instead of handing that credential to browser JavaScript through ws-token.
 */
export function carriesLegacyApiKey(token: string): boolean {
  const value = decodeJwtPayload(token).core_api_key;
  return typeof value === "string" && value.length > 0;
}
