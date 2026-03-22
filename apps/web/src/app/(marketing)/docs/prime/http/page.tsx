import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Prime HTTP API",
  description: "Use AllSource Prime from any language via REST. Full endpoint reference with curl examples.",
});

export default function HttpPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        HTTP API
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        Use Prime from any language via REST. Start the server and call endpoints with curl, fetch, or any HTTP client.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-10">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Start the Server
          </h2>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
{`allsource-prime --mode http --port 3905 --data-dir ~/.prime/memory`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            The server binds to <code>0.0.0.0:3905</code> by default. All data is persisted
            to the specified data directory.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Create a Node
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>POST /api/v1/prime/nodes</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl -X POST http://localhost:3905/api/v1/prime/nodes \\
  -H "Content-Type: application/json" \\
  -d '{
    "type": "person",
    "name": "Alice",
    "domain": "engineering",
    "properties": {
      "role": "Lead Engineer",
      "started": "2026-01"
    }
  }'`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "id": "node_a1b2c3d4",
  "type": "person",
  "name": "Alice",
  "domain": "engineering",
  "properties": { "role": "Lead Engineer", "started": "2026-01" },
  "created_at": "2026-03-22T10:00:00Z"
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Get a Node
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>GET /api/v1/prime/nodes/:id</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl http://localhost:3905/api/v1/prime/nodes/node_a1b2c3d4`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "id": "node_a1b2c3d4",
  "type": "person",
  "name": "Alice",
  "domain": "engineering",
  "properties": { "role": "Lead Engineer", "started": "2026-01" },
  "created_at": "2026-03-22T10:00:00Z"
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Delete a Node
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>DELETE /api/v1/prime/nodes/:id</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl -X DELETE http://localhost:3905/api/v1/prime/nodes/node_a1b2c3d4`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{ "deleted": true, "id": "node_a1b2c3d4" }`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Deletes are soft — the node is removed from the active graph but preserved in the event history.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Create an Edge
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>POST /api/v1/prime/edges</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl -X POST http://localhost:3905/api/v1/prime/edges \\
  -H "Content-Type: application/json" \\
  -d '{
    "source": "node_a1b2c3d4",
    "target": "node_e5f6g7h8",
    "relation": "leads"
  }'`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "source": "node_a1b2c3d4",
  "target": "node_e5f6g7h8",
  "relation": "leads",
  "created_at": "2026-03-22T10:01:00Z"
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Get Neighbors
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>GET /api/v1/prime/nodes/:id/neighbors</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl http://localhost:3905/api/v1/prime/nodes/node_a1b2c3d4/neighbors`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "neighbors": [
    {
      "node": { "id": "node_e5f6g7h8", "type": "project", "name": "Prime" },
      "relation": "leads",
      "direction": "outgoing"
    }
  ],
  "count": 1
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Hybrid Recall
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>POST /api/v1/prime/recall</code> — combines vector similarity, graph expansion, and temporal recency.
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl -X POST http://localhost:3905/api/v1/prime/recall \\
  -H "Content-Type: application/json" \\
  -d '{
    "query": "Who leads the Prime project?",
    "top_k": 5
  }'`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "results": [
    {
      "node": { "id": "node_a1b2c3d4", "type": "person", "name": "Alice" },
      "score": 0.92,
      "context": [
        { "relation": "leads", "target": "Prime" }
      ]
    }
  ],
  "query": "Who leads the Prime project?"
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Compressed Index
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>GET /api/v1/prime/index</code> — returns the auto-generated markdown knowledge map.
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl http://localhost:3905/api/v1/prime/index`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "index": "# Knowledge Index\\n_2 nodes, 1 domain_\\n\\n## engineering\\n- **Nodes:** 2 (person, project)\\n- **Examples:** Alice, Prime",
  "node_count": 2,
  "domain_count": 1
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Combined Context
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>POST /api/v1/prime/context</code> — recall + index scaffolding for cross-domain questions.
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl -X POST http://localhost:3905/api/v1/prime/context \\
  -H "Content-Type: application/json" \\
  -d '{ "query": "How does engineering relate to revenue?" }'`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "index_excerpt": "## engineering\\n- ...",
  "recall_results": [...],
  "cross_references": [
    { "from_domain": "engineering", "to_domain": "revenue", "edges": 3 }
  ]
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Graph Stats
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>GET /api/v1/prime/stats</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl http://localhost:3905/api/v1/prime/stats`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{
  "nodes": 42,
  "edges": 67,
  "domains": ["engineering", "product", "revenue"],
  "vectors": 128,
  "events_total": 312
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Health Check
          </h2>
          <p className="text-muted-foreground mb-3">
            <code>GET /health</code>
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`curl http://localhost:3905/health`}
          </pre>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto mt-2">
{`{ "status": "ok" }`}
          </pre>
          <p className="text-sm text-muted-foreground mt-2">
            Note: the health endpoint is at the root path, not under <code>/api/v1/</code>.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            CORS Configuration
          </h2>
          <p className="text-muted-foreground leading-relaxed">
            The HTTP server enables CORS for all origins by default in development.
            For production, set the <code>PRIME_CORS_ORIGINS</code> environment variable
            to a comma-separated list of allowed origins:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto mt-3">
{`PRIME_CORS_ORIGINS="https://app.example.com,https://admin.example.com" \\
  allsource-prime --mode http --port 3905 --data-dir /data/prime`}
          </pre>
        </section>
      </div>
    </div>
  );
}
