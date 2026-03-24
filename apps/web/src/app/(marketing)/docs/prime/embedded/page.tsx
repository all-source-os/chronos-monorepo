import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Prime Embedded (Rust Library)",
  description: "Use AllSource Prime as an embedded Rust library for knowledge graphs, vector search, and hybrid recall.",
  canonical: "/docs/prime/embedded",
});

export default function EmbeddedPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        Embedded (Rust Library)
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        Use Prime directly as a Rust crate for maximum performance. No network overhead,
        no server process — just a library call.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-10">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Add the Dependency
          </h2>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto">
{`# Cargo.toml
[dependencies]
allsource-core = { version = "0.16", features = ["prime"] }`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Feature Flags
          </h2>
          <p className="text-muted-foreground mb-3">
            Enable only what you need:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead><tr className="border-b">
                <th className="p-2 text-left">Feature</th>
                <th className="p-2 text-left">What it includes</th>
              </tr></thead>
              <tbody>
                <tr className="border-b border-muted"><td className="p-2"><code>prime</code></td><td className="p-2">Knowledge graph (nodes, edges, domains, compressed index)</td></tr>
                <tr className="border-b border-muted"><td className="p-2"><code>prime-vectors</code></td><td className="p-2">Vector storage and HNSW similarity search</td></tr>
                <tr className="border-b border-muted"><td className="p-2"><code>prime-recall</code></td><td className="p-2">Hybrid recall engine (vector + graph + temporal)</td></tr>
                <tr className="border-b border-muted"><td className="p-2"><code>prime-full</code></td><td className="p-2">All of the above combined</td></tr>
              </tbody>
            </table>
          </div>
          <pre className="rounded-lg bg-black/80 p-4 text-sm text-green-400 overflow-x-auto mt-3">
{`# Enable everything
allsource-core = { version = "0.16", features = ["prime-full"] }`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Knowledge Graph Basics
          </h2>
          <p className="text-muted-foreground mb-3">
            Open a persistent graph, add nodes and edges, query neighbors:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`use allsource_core::prime::Prime;

fn main() -> anyhow::Result<()> {
    // Open a persistent graph (creates the directory if needed)
    let prime = Prime::open("./my-memory")?;

    // Add nodes
    let alice = prime.add_node("person", "Alice", "engineering", json!({
        "role": "Lead Engineer",
        "started": "2026-01"
    }))?;

    let project = prime.add_node("project", "Prime", "engineering", json!({
        "status": "active"
    }))?;

    // Connect them
    prime.add_edge(&alice.id, &project.id, "leads")?;

    // Query neighbors
    let neighbors = prime.neighbors(&alice.id)?;
    for n in &neighbors {
        println!("{} --{}-> {}", alice.name, n.relation, n.node.name);
    }
    // Output: Alice --leads-> Prime

    // Graceful shutdown (flushes WAL, writes Parquet)
    prime.shutdown()?;
    Ok(())
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Vector Search
          </h2>
          <p className="text-muted-foreground mb-3">
            Store embeddings and find semantically similar content. Requires the{" "}
            <code>prime-vectors</code> feature:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`use allsource_core::prime::Prime;

let prime = Prime::open("./my-memory")?;

// Store a vector with text and metadata
let embedding: Vec<f32> = your_embedding_model("Alice leads the Prime project");
prime.embed(embedding, "Alice leads the Prime project", json!({
    "source": "conversation",
    "timestamp": "2026-03-22T10:00:00Z"
}))?;

// Find similar content
let query_vec: Vec<f32> = your_embedding_model("who works on Prime?");
let results = prime.similar(query_vec, 5)?; // top 5

for result in &results {
    println!("score={:.3} text={}", result.score, result.text);
}`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Hybrid Recall
          </h2>
          <p className="text-muted-foreground mb-3">
            Combine vector similarity, graph expansion, and temporal recency into a single
            query. Requires the <code>prime-recall</code> feature:
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
{`use allsource_core::prime::{Prime, RecallEngine};

let prime = Prime::open("./my-memory")?;
let recall = RecallEngine::new(&prime);

// Hybrid recall — vector + graph + temporal
let results = recall.recall("Who leads the Prime project?", 5)?;
for r in &results {
    println!("{}: score={:.3}", r.node.name, r.score);
}

// Build a compressed index (markdown TOC of all knowledge)
let index = recall.build_heuristic_index()?;
println!("{}", index);
// # Knowledge Index
// _2 nodes, 1 domain_
//
// ## engineering
// - **Nodes:** 2 (person, project)
// - **Examples:** Alice, Prime

// Raw summary for custom formatting
let summary = recall.build_raw_summary()?;
println!("domains: {:?}, nodes: {}", summary.domains, summary.total_nodes);`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            Persistence
          </h2>
          <p className="text-muted-foreground leading-relaxed">
            <code>Prime::open(&quot;path&quot;)</code> opens a durable store backed by WAL (Write-Ahead Log)
            and Parquet. Data survives crashes and restarts:
          </p>
          <ul className="space-y-1 text-sm text-muted-foreground mt-3">
            <li><strong>WAL:</strong> CRC32 checksums, configurable fsync (default 100ms), automatic crash recovery</li>
            <li><strong>Parquet:</strong> Periodic flush with Snappy compression for long-term columnar storage</li>
            <li><strong>DashMap:</strong> In-memory concurrent map rebuilt from WAL on startup for fast reads</li>
          </ul>
          <p className="text-muted-foreground leading-relaxed mt-3">
            Call <code>prime.shutdown()</code> for graceful shutdown. On crash, WAL replay
            recovers all committed events automatically.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">
            When to Use Each Mode
          </h2>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead><tr className="border-b">
                <th className="p-2 text-left">Mode</th>
                <th className="p-2 text-left">Best for</th>
                <th className="p-2 text-left">Latency</th>
              </tr></thead>
              <tbody>
                <tr className="border-b border-muted">
                  <td className="p-2"><strong>Embedded</strong></td>
                  <td className="p-2">Rust applications, CLIs, maximum performance</td>
                  <td className="p-2">~12&micro;s per query</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2"><strong>MCP</strong></td>
                  <td className="p-2">Claude Desktop, AI agent workflows</td>
                  <td className="p-2">~1ms (stdio)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2"><strong>HTTP</strong></td>
                  <td className="p-2">Any language, microservices, web apps</td>
                  <td className="p-2">~2-5ms (network)</td>
                </tr>
              </tbody>
            </table>
          </div>
          <p className="text-sm text-muted-foreground mt-3">
            Use embedded when Prime is a core part of your Rust application.
            Use MCP for AI agent integrations. Use HTTP when you need language-agnostic access
            or want to share a single Prime instance across multiple clients.
          </p>
        </section>
      </div>
    </div>
  );
}
