import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "MCP Integration — Connect securely to production",
  description: `Connect ${siteConfig.name} to Claude Desktop, Cursor, or any MCP client and reach your production event data securely through the authenticated gateway.`,
  canonical: "/docs/mcp",
});

const GATEWAY = "https://api.all-source.xyz";

function CodeBlock({ children }: { children: string }) {
  return (
    <pre className="rounded-lg border border-border bg-muted/50 p-4 text-sm overflow-x-auto">
      <code>{children}</code>
    </pre>
  );
}

export default function McpPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">MCP Integration</h1>
      <p className="text-lg text-muted-foreground mb-10">
        AllSource ships a Model Context Protocol (MCP) connector so AI agents — Claude Desktop,
        Cursor, or anything that speaks MCP — can ingest, query, and reason over your event store in
        natural language. This page shows how to connect to your <strong>production</strong> data{" "}
        <strong>securely</strong>, and how that differs from the throwaway localhost setup.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-10">
        {/* ── Security model first ─────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            How a secure connection is shaped
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            The MCP connector never talks to the database (Core) directly. Every request is mediated
            by the authenticated gateway at{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">{GATEWAY}</code>, which
            validates your API key, scopes the call to your tenant, and enforces quotas and rate
            limits. Core is internal-only and trusts any caller on its network — so it is never
            exposed to the public, and neither your agent nor the connector should ever point at it.
          </p>
          <CodeBlock>{`Claude Desktop / Cursor
        │  (MCP — local, stdio or localhost SSE)
        ▼
AllSource MCP connector            ← runs on YOUR machine / your infra
        │  HTTPS + Authorization: Bearer <serviceaccount key>
        ▼
${GATEWAY}  (gateway)               ← validates key, derives tenant_id
        │  internal network only
        ▼
AllSource Core (your tenant's events only)`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Three properties make this safe: the key is a{" "}
            <strong>least-privilege, tenant-scoped</strong> credential (a scoped key can only ever
            read/write its own tenant&apos;s events); the transport to the gateway is{" "}
            <strong>TLS</strong>; and the connector runs where <em>you</em> control it, so your key
            never leaves your environment.
          </p>
        </section>

        {/* ── Step 1: mint a key ───────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            1. Mint a least-privilege API key
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            An MCP agent needs to read and write events — nothing more. Mint a key with the{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">serviceaccount</code> role: it
            is granted read + write and is denied admin, tenant management, metrics, and schema
            administration. Do <strong>not</strong> use an admin JWT or an admin-role key as your
            connector credential.
          </p>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Self-service (creates a tenant and returns a scoped key in one call):
          </p>
          <CodeBlock>{`curl -X POST ${GATEWAY}/api/v1/onboard/start \\
  -H "Content-Type: application/json" \\
  -d '{"name": "my-agent"}'

# →
# {
#   "tenant_id": "my-agent-a1b2c3",
#   "api_key": "eyJhbGciOiJIUzI1NiIs...",   ← store this in a secret, shown once
#   ...
# }`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed my-3">
            Already onboarded? Mint a fresh, role-scoped key with an admin token:
          </p>
          <CodeBlock>{`curl -X POST ${GATEWAY}/api/v1/teams/agent-keys \\
  -H "Authorization: Bearer $ADMIN_JWT" \\
  -H "Content-Type: application/json" \\
  -d '{"name": "claude-desktop", "role": "serviceaccount"}'`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3 text-sm">
            The role string is exactly{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">serviceaccount</code> — no
            underscore. A drifted{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">service_account</code> is
            rejected and every request silently 403s.
          </p>
        </section>

        {/* ── Step 2: run the connector against prod ───────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            2. Run the MCP connector against production
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Run the connector yourself and point it at the gateway with your key. The two knobs that
            matter: <code className="text-sm bg-muted px-1.5 py-0.5 rounded">CORE_URL</code> (the
            gateway, <strong>not</strong> Core) and{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">CORE_API_KEY</code> (your
            scoped key, supplied as a secret — never hard-coded).
          </p>
          <CodeBlock>{`docker run -p 3904:3904 \\
  -e CORE_URL=${GATEWAY} \\
  -e CORE_API_KEY=$ALLSOURCE_API_KEY \\
  ghcr.io/all-source-os/chronos-mcp:latest`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3">
            With those set, the connector authenticates to the gateway on every call over HTTPS; the
            gateway resolves your tenant from the key and returns only your tenant&apos;s data. Pass
            the key from your shell or a secrets manager (
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">$ALLSOURCE_API_KEY</code>), so
            it is never written into an image, a compose file, or your shell history.
          </p>
        </section>

        {/* ── Step 3: wire the client ──────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            3. Point your MCP client at the connector
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Your AI client connects to the connector running on your machine — it never holds the
            API key itself. Add this to your{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">
              claude_desktop_config.json
            </code>
            :
          </p>
          <CodeBlock>{`{
  "mcpServers": {
    "allsource": {
      "url": "http://localhost:3904/sse"
    }
  }
}`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3">
            The client → connector hop is local (loopback); the connector → gateway hop is the
            authenticated, TLS one. Your production credential lives only in the connector&apos;s
            environment, not in the client config.
          </p>
        </section>

        {/* ── Verify ───────────────────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            4. Verify the connection is tenant-scoped
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            Before trusting the agent, confirm the key reaches your data and only your data. The
            same credential against the gateway REST API:
          </p>
          <CodeBlock>{`curl "${GATEWAY}/api/v1/events/query?limit=1" \\
  -H "Authorization: Bearer $ALLSOURCE_API_KEY"
# → {"events": [ ... ], "count": N}   ← your tenant's events only`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Then ask the agent to run a query through MCP and check the result matches. A scoped key
            cannot read another tenant&apos;s events — the gateway derives the tenant from the key,
            not from any field the caller supplies.
          </p>
        </section>

        {/* ── Hardening checklist ──────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Production hardening checklist
          </h2>
          <ul className="space-y-2 text-muted-foreground leading-relaxed list-none pl-0">
            {[
              [
                "Use the gateway, never Core",
                `Always set CORE_URL=${GATEWAY}. Core is internal-only and trusts any caller on its network — pointing an external connector at it bypasses auth, quotas, and tenant isolation entirely.`,
              ],
              [
                "serviceaccount role, least privilege",
                "Mint the connector key as serviceaccount (read + write). Reserve admin keys for humans; never hand an admin credential to an agent.",
              ],
              [
                "Key in a secret, not in config",
                "Inject CORE_API_KEY from an environment secret or secrets manager. Keep it out of images, compose files, claude_desktop_config.json, and git.",
              ],
              [
                "TLS only",
                "CORE_URL must be https://. The connector → gateway hop carries your key — never run it over plain http in production.",
              ],
              [
                "Rotate and revoke",
                "Rotate the key on a schedule and immediately if a machine running the connector is lost. Revoke from the dashboard; a revoked key fails closed.",
              ],
              [
                "One key per connector",
                "Give each connector / machine its own key so you can revoke a single one without taking down the rest, and so audit trails stay attributable.",
              ],
            ].map(([title, desc]) => (
              <li key={title} className="rounded-lg border border-border p-4">
                <span className="block font-medium text-foreground mb-1">{title}</span>
                <span className="text-sm">{desc}</span>
              </li>
            ))}
          </ul>
        </section>

        {/* ── Example tools ────────────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">What the agent can do</h2>
          <p className="text-muted-foreground leading-relaxed mb-4">
            Through the connector your agent gets the event-store toolset — all scoped to your
            tenant by the key:
          </p>
          <div className="space-y-3">
            {[
              { name: "ingest_event", desc: "Store a new event in a stream" },
              { name: "query_events", desc: "Query events by stream, type, time range, or entity" },
              { name: "create_projection", desc: "Build a materialized view over event streams" },
              { name: "get_stream_stats", desc: "Event counts and throughput for a stream" },
              { name: "register_schema", desc: "Register a JSON schema for event validation" },
              { name: "health_check", desc: "Check Core and system health" },
            ].map((tool) => (
              <div
                key={tool.name}
                className="flex items-start gap-3 rounded-lg border border-border p-3"
              >
                <code className="text-sm font-mono text-primary shrink-0">{tool.name}</code>
                <span className="text-sm text-muted-foreground">{tool.desc}</span>
              </div>
            ))}
          </div>
        </section>

        {/* ── Troubleshooting ──────────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Troubleshooting</h2>
          <ul className="space-y-2 text-muted-foreground leading-relaxed list-none pl-0">
            {[
              [
                "401 from the gateway",
                "No / invalid key. Confirm CORE_API_KEY is set in the connector's environment and the key hasn't been revoked or expired.",
              ],
              [
                "403 on every call",
                "Role drift. The key's role must be the exact string serviceaccount (no underscore). Re-mint with the correct role.",
              ],
              [
                "Empty results but data exists",
                "Wrong tenant or wrong base URL. Verify CORE_URL is the gateway and the key belongs to the tenant that owns the data — the gateway scopes by key, so a key from another tenant returns nothing, not an error.",
              ],
              [
                "Connector can't reach the gateway",
                `Check egress/TLS from the machine running the connector to ${GATEWAY}. Do not work around it by pointing CORE_URL at Core.`,
              ],
            ].map(([title, desc]) => (
              <li key={title} className="rounded-lg border border-border p-4">
                <span className="block font-medium text-foreground mb-1">{title}</span>
                <span className="text-sm">{desc}</span>
              </li>
            ))}
          </ul>
        </section>

        {/* ── Local dev contrast ───────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Local development (no auth — never for production)
          </h2>
          <p className="text-muted-foreground leading-relaxed mb-3">
            When you run the full AllSource stack locally, the connector points at your local Core
            with no key. This is convenient for development but has{" "}
            <strong>no authentication and no tenant isolation</strong> — only ever use it against a
            local Core, never against production:
          </p>
          <CodeBlock>{`# LOCAL ONLY — local Core, no auth. Do not use these values for production.
docker run -p 3904:3904 \\
  -e CORE_URL=http://host.docker.internal:3900 \\
  ghcr.io/all-source-os/chronos-mcp:latest`}</CodeBlock>
          <p className="text-muted-foreground leading-relaxed mt-3 text-sm">
            The difference between this and a secure production connection is exactly the two things
            above: <code className="text-sm bg-muted px-1.5 py-0.5 rounded">CORE_URL</code> pointed
            at the gateway instead of a local Core, and a scoped{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">CORE_API_KEY</code>.
          </p>
        </section>

        {/* ── Honest status note ───────────────────────────────────────── */}
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">A note on hosted MCP</h2>
          <p className="text-muted-foreground leading-relaxed">
            Today you run the MCP connector yourself (locally or on your own infrastructure) and it
            reaches production through the gateway — there is no public, multi-tenant hosted MCP URL
            to point a client at directly. If you only need to store and query events and don&apos;t
            want to run a connector at all, the gateway REST API and the{" "}
            <a
              href="/docs"
              className="text-foreground underline underline-offset-4 hover:opacity-80"
            >
              SDKs
            </a>{" "}
            give you the same tenant-scoped access with the same{" "}
            <code className="text-sm bg-muted px-1.5 py-0.5 rounded">serviceaccount</code> key.
          </p>
        </section>
      </div>
    </div>
  );
}
