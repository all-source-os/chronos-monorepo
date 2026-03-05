import { expect, type APIResponse } from "@playwright/test";

/**
 * Assert response is OK (2xx), with a descriptive failure message
 * showing the actual status code and response body.
 */
export async function expectOk(resp: APIResponse, context?: string) {
  if (!resp.ok()) {
    const body = await resp.text().catch(() => "<no body>");
    const prefix = context ? `${context}: ` : "";
    throw new Error(
      `${prefix}expected 2xx but got ${resp.status()} ${resp.statusText()}\n${body}`
    );
  }
}
