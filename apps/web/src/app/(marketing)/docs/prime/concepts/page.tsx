import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Prime Concepts",
  description:
    "Core concepts: compressed index, domains, cross-domain reasoning, temporal queries, event provenance.",
  canonical: "/docs/prime/concepts",
});

export default function ConceptsPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      <h1 className="text-3xl font-bold text-foreground sm:text-4xl mb-2">
        How Prime memory works
      </h1>
      <p className="text-lg text-muted-foreground mb-10">
        The ideas behind Prime&apos;s agent memory architecture.
      </p>

      <div className="prose prose-neutral dark:prose-invert max-w-none space-y-10">
        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Everything is an Event</h2>
          <p className="text-muted-foreground leading-relaxed">
            Every mutation — creating a node, adding an edge, storing a vector, deleting a memory —
            is an immutable event in a Write-Ahead Log (WAL). Events are never modified or deleted.
            This gives you full audit trails, time-travel queries, and crash recovery for free.
          </p>
          <pre className="rounded-lg bg-black/80 p-4 text-xs text-green-400 overflow-x-auto">
            {`prime.node.created  → { id, type, properties, domain }
prime.edge.created  → { source, target, relation }
prime.vector.stored → { embedding, text, metadata }
prime.node.deleted  → { } (soft-delete, preserved in history)`}
          </pre>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Projections</h2>
          <p className="text-muted-foreground leading-relaxed">
            Projections are materialized views built incrementally from the event stream. Each
            projection maintains a different index over the same data:
          </p>
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b">
                  <th className="p-2 text-left">Projection</th>
                  <th className="p-2 text-left">What it indexes</th>
                  <th className="p-2 text-left">Query cost</th>
                </tr>
              </thead>
              <tbody>
                <tr className="border-b border-muted">
                  <td className="p-2">NodeState</td>
                  <td className="p-2">Node properties by entity_id</td>
                  <td className="p-2">O(1)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">AdjacencyList</td>
                  <td className="p-2">Outgoing edges per node</td>
                  <td className="p-2">O(1)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">VectorIndex</td>
                  <td className="p-2">HNSW over embeddings</td>
                  <td className="p-2">O(log n)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">DomainIndex</td>
                  <td className="p-2">Nodes grouped by domain</td>
                  <td className="p-2">O(1)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">CrossDomain</td>
                  <td className="p-2">Edges spanning domain boundaries</td>
                  <td className="p-2">O(1)</td>
                </tr>
                <tr className="border-b border-muted">
                  <td className="p-2">CompressedIndex</td>
                  <td className="p-2">Auto-generated markdown TOC</td>
                  <td className="p-2">Cached</td>
                </tr>
              </tbody>
            </table>
          </div>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Domains</h2>
          <p className="text-muted-foreground leading-relaxed">
            Every node can be tagged with a <strong>domain</strong> — a knowledge area like
            &quot;engineering&quot;, &quot;revenue&quot;, or &quot;product&quot;. Domains power the
            compressed index and cross-domain reasoning:
          </p>
          <ul className="space-y-1 text-sm text-muted-foreground mt-3">
            <li>The compressed index organizes knowledge by domain</li>
            <li>Cross-domain edges (e.g., revenue → engineering) generate cross-references</li>
            <li>Questions spanning domains trigger both the index and vector search</li>
          </ul>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Compressed Index</h2>
          <p className="text-muted-foreground leading-relaxed">
            A token-efficient markdown summary (~500-1000 tokens) of everything the agent knows.
            Organized by domain with cross-references. Auto-generated from projections — no manual
            maintenance.
          </p>
          <p className="text-muted-foreground leading-relaxed mt-2">
            This is the key insight from{" "}
            <a href="https://github.com/roli-lpci/zer0dex" className="text-foreground underline">
              zer0dex
            </a>
            : vector similarity finds X <em>or</em> Y when you ask &quot;how does X relate to
            Y?&quot; — rarely both. The compressed index bridges this gap by providing explicit
            cross-domain pointers, doubling cross-reference accuracy from 37.5% to 80%.
          </p>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Temporal Queries</h2>
          <p className="text-muted-foreground leading-relaxed">
            Since everything is an event, time-travel is a first-class operation:
          </p>
          <ul className="space-y-1 text-sm text-muted-foreground mt-3">
            <li>
              <code>get_node_as_of(id, timestamp)</code> — reconstruct state at any past point
            </li>
            <li>
              <code>history(id)</code> — full audit trail with timestamps
            </li>
            <li>
              <code>diff(from, to)</code> — what changed between two timestamps
            </li>
            <li>
              <code>neighbors_as_of(id, timestamp)</code> — graph state at a past point
            </li>
          </ul>
        </section>

        <section>
          <h2 className="text-xl font-semibold text-foreground mb-3">Hybrid Recall</h2>
          <p className="text-muted-foreground leading-relaxed">
            <code>prime_recall</code> combines three retrieval strategies:
          </p>
          <ol className="space-y-1 text-sm text-muted-foreground mt-3 list-decimal list-inside">
            <li>
              <strong>Vector similarity</strong> — find semantically related facts via HNSW
            </li>
            <li>
              <strong>Graph expansion</strong> — follow edges from vector matches to discover
              connected context
            </li>
            <li>
              <strong>Temporal recency</strong> — boost recently stored facts
            </li>
          </ol>
          <p className="text-muted-foreground leading-relaxed mt-3">
            For cross-domain questions, use <code>prime_context</code> instead — it adds the
            compressed index excerpt for navigational scaffolding.
          </p>
        </section>
      </div>
    </div>
  );
}
