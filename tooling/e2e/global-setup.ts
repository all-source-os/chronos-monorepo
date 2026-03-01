/**
 * Playwright Global Setup — Health Check Gate
 *
 * Verifies all required services are healthy before any tests run.
 * Fails fast with a clear error message if any service is unreachable.
 *
 * Required services:
 *   - Web app (BASE_URL, default http://localhost:3000)
 *   - Control Plane (CONTROL_PLANE_URL, default http://localhost:3901)
 *
 * Optional services (checked but warn-only when targeting production):
 *   - Core (CORE_URL, default http://localhost:3900)
 *   - Query Service (QS_URL, default http://localhost:3902)
 */

const BASE_URL = process.env.BASE_URL || "https://all-source.xyz";
const CP_URL = process.env.CONTROL_PLANE_URL || "http://localhost:3901";
const CORE_URL = process.env.CORE_URL || "http://localhost:3900";
const QS_URL =
  process.env.QS_URL ||
  process.env.NEXT_PUBLIC_API_URL ||
  "http://localhost:3902";

interface ServiceCheck {
  name: string;
  url: string;
  healthPath: string;
  required: boolean;
}

async function checkHealth(
  service: ServiceCheck
): Promise<{ ok: boolean; status?: number; error?: string }> {
  const url = `${service.url}${service.healthPath}`;
  try {
    const resp = await fetch(url, {
      signal: AbortSignal.timeout(5000),
    });
    if (resp.ok) return { ok: true, status: resp.status };
    return { ok: false, status: resp.status, error: `HTTP ${resp.status}` };
  } catch (err: any) {
    const msg =
      err?.cause?.code || err?.code || err?.message || String(err);
    return { ok: false, error: msg };
  }
}

export default async function globalSetup() {
  const isLocal = BASE_URL.includes("localhost");

  const services: ServiceCheck[] = [
    {
      name: "Web App",
      url: BASE_URL,
      healthPath: "/",
      required: true,
    },
    {
      name: "Control Plane",
      url: CP_URL,
      healthPath: "/health",
      required: true,
    },
  ];

  // Core and QS are only checked for local runs — in production they're
  // behind the QS/CP and not directly reachable by the browser.
  if (isLocal) {
    services.push(
      {
        name: "Core",
        url: CORE_URL,
        healthPath: "/health",
        required: true,
      },
      {
        name: "Query Service",
        url: QS_URL,
        healthPath: "/api/health",
        required: true,
      }
    );
  }

  console.log("\n🏥 Checking service health before running tests...\n");

  const results = await Promise.all(
    services.map(async (svc) => {
      const result = await checkHealth(svc);
      const icon = result.ok ? "✅" : svc.required ? "❌" : "⚠️";
      const detail = result.ok
        ? `healthy (${result.status})`
        : result.error || "unknown error";
      console.log(`  ${icon} ${svc.name.padEnd(20)} ${svc.url}${svc.healthPath} — ${detail}`);
      return { ...svc, ...result };
    })
  );

  console.log("");

  const failed = results.filter((r) => !r.ok && r.required);
  if (failed.length > 0) {
    const names = failed.map((f) => f.name).join(", ");
    throw new Error(
      `Cannot run e2e tests: ${failed.length} required service(s) are down: ${names}.\n` +
        `Start the stack first: docker compose up -d (or see docs/deployment/DOCKER.md)`
    );
  }
}
