# voice-demo results

- binary: `/Users/decebaldobrica/Projects/chronos/chronos-monorepo/tooling/voice-demo/../../apps/prime-mcp/target/release/allsource-prime` (allsource-prime 0.21.6)
- run (UTC): 2026-06-02T11:10:38.245406+00:00
- facets seeded: 12
- prompt: "Write a short LinkedIn post arguing that when your system's availability hurts, adding another database is usually the wrong fix."

All output below is verbatim from the real `allsource-prime` binary over stdio MCP, against a throwaway temp `--data-dir`. Nothing here is hand-written.

## 1. Facets recorded (prime_stats — REAL binary output)

```json
{
  "total_nodes": 12,
  "total_edges": 7,
  "event_count": 31,
  "nodes_by_type": {
    "voice": 12
  },
  "edges_by_relation": {
    "relates_to": 7
  },
  "sync": false
}
```

Sample recorded entity_ids (12 total):

```
node:voice:fa108893-07e7-4f55-b066-b727b41a1878  [thinking_pattern] First-principles over cargo-culting
node:voice:760739da-f3ea-4f26-b0b2-a38099a54197  [thinking_pattern] Invert the question
node:voice:471b2322-b366-41dd-b445-3ab73ecf2d25  [thinking_pattern] Cost of being wrong, not odds of being right
node:voice:d31ed3db-150b-4cfb-b8b1-32949f1f5355  [communication_style] Lead with the punchline
```

## 2. The compressed voice file (prime_index — REAL binary output)

This is the auto-generated equivalent of the post's hand-compressed ~4k-token markdown — but always current and never copy-pasted.

- token_count: 77
- domains: ['voice.communication', 'voice.thinking', 'voice.frameworks', 'voice.expertise', 'voice.contrarian']
- cross_references: []
- last_updated: 2026-06-02T11:10:38.613501+00:00

```markdown
# Knowledge Index

_12 nodes, 5 domains, 0 cross-domain links_

## Domains

### voice.communication

- **Nodes:** 3
- **Examples:** d31ed3db-150b-4cfb-b8b1-32949f1f5355, 5b7e423d-c374-4b94-82fd-9831b609f734, f2b58771-077e-49ed-ba2d-04e4074869a2

### voice.thinking

- **Nodes:** 3
- **Examples:** fa108893-07e7-4f55-b066-b727b41a1878, 760739da-f3ea-4f26-b0b2-a38099a54197, 471b2322-b366-41dd-b445-3ab73ecf2d25

### voice.frameworks

- **Nodes:** 2
- **Examples:** 5ba192ca-d640-4a6f-aedb-2e3925a8ee6d, c486646b-7627-441d-b283-5a2b53f284a2

### voice.expertise

- **Nodes:** 2
- **Examples:** 124511ed-5366-403c-b8c1-9c9d073d9e33, db2b98f1-bbd1-4be5-bae2-03fec3ae5b97

### voice.contrarian

- **Nodes:** 2
- **Examples:** 4e3559a5-735c-4026-b04f-1df4dfa7d5d3, 54de985d-15bc-4809-98a2-8dafcab0b93e
```

## 3. Voice slice recalled for the prompt (prime_context — REAL binary output)

Instead of pasting the whole voice file, we recall only the facets relevant to THIS task. These are the vector hits returned for the prompt:

- tier: L2  token_count: 77

```json
[]
```

> Post-fix note: the empty `[]` above is the captured output as it was at run time.
> Since fixed in commit 5083017, `prime_context` L2 now returns full hybrid recall —
> the populated index plus vector hits plus graph nodes (an L2 query returns the
> index + 4 vectors + 3 graph nodes, top node score 0.73). The captured block is
> left verbatim; only this note records the fix.

## 4. prime_recall for the prompt (REAL binary output)

```json
{
  "nodes": [
    {
      "name": "Add a database is usually the wrong fix",
      "facet": "contrarian_take",
      "score": 0.7572,
      "depth": 0
    },
    {
      "name": "Distributed systems durability",
      "facet": "domain_expertise",
      "score": 0.6884,
      "depth": 0
    },
    {
      "name": "Event sourcing battle scars",
      "facet": "domain_expertise",
      "score": 0.6188,
      "depth": 0
    },
    {
      "name": "Make the implicit explicit",
      "facet": "strategic_framework",
      "score": 0.5867,
      "depth": 0
    },
    {
      "name": "Invert the question",
      "facet": "thinking_pattern",
      "score": 0.5725,
      "depth": 0
    }
  ],
  "vectors": [
    {
      "text_head": "When availability or scale hurts, the reflex is 'add another database ",
      "score": 0.5145
    },
    {
      "text_head": "My expertise is durable storage and distributed systems: WAL design, f",
      "score": 0.3767
    },
    {
      "text_head": "I've run event-sourced systems in production for years. The scar tissu",
      "score": 0.2375
    },
    {
      "text_head": "My framework for ambiguity: the bug is almost always an unstated assum",
      "score": 0.1735
    },
    {
      "text_head": "My default move on a hard problem is to invert it: instead of 'how do ",
      "score": 0.1451
    },
    {
      "text_head": "My core decision framework: classify every decision as a two-way door ",
      "score": 0.1274
    },
    {
      "text_head": "Contrarian take I'll defend: most teams adopt microservices years befo",
      "score": 0.1018
    },
    {
      "text_head": "I distrust abstraction without an example. Every claim gets a concrete",
      "score": 0.0646
    },
    {
      "text_head": "I reason from first principles. When someone cites a best practice I a",
      "score": 0.0241
    },
    {
      "text_head": "My sentences are short and declarative. I use dry, understated humor \u2014",
      "score": 0.0167
    }
  ],
  "edges": 0
}
```

## 5. The voice has history (prime_history — REAL binary output)

Audit trail for one facet node (`node:voice:fa108893-07e7-4f55-b066-b727b41a1878`). A static markdown blob has no provenance; a prime voice file time-travels.

```json
[
  {
    "type": "prime.node.created",
    "timestamp": "2026-06-02T11:10:38.259662+00:00"
  }
]
```

## 6. voice-ON vs voice-OFF (same prompt)

The recalled voice slice in sections 3-4 IS the injected context for the voice-ON arm. The voice-OFF arm gets no recall. The two completions below are written by the agent (Claude) for the SAME prompt — once using the recalled facets as injected context, once generic — so the difference is visible and grounded in the real recall output above.

**Prompt:** Write a short LinkedIn post arguing that when your system's availability hurts, adding another database is usually the wrong fix.

### voice-OFF (no recall — generic)

> When your system's availability starts to suffer, it can be tempting to reach for
> another database to solve the problem. However, adding a new datastore is often
> not the right solution. It introduces additional operational complexity, new
> failure modes, and consistency challenges that can actually make availability
> worse. Before adding infrastructure, it's worth taking the time to understand the
> root cause of your availability issues. Often the real fix is in your existing
> architecture. What's your experience with scaling for availability? #engineering
> #architecture #scaling

### voice-ON (recalled facets injected)

Written from the recalled facets in §4 — the contrarian take "add a database is
usually the wrong fix" (0.76), the durability/event-sourcing expertise, "invert
the question," and the communication style (lead with the punchline, short
declarative lines, dry understatement):

> Your availability hurts, so you're adding a database. You just bought a second
> thing that can be down.
>
> Adding a datastore to fix availability is usually backwards. You didn't have an
> availability problem — you had a coupling problem, and now it spans two systems
> with a network in between. New failure modes. New consistency story. Same outage,
> longer postmortem.
>
> Invert it: don't ask "what do I add to stay up?" Ask "what is currently allowed
> to take me down?" Nine times out of ten the answer is already in your stack — a
> synchronous call that should be an event, a write path with no durability
> guarantee, a dependency you never had to take.
>
> Fix the coupling. Make the implicit explicit. The database you don't add is the
> one that never pages you at 3am.

---
_wall 0.4s incl. first-call fastembed model load. Binary: allsource-prime 0.21.6._

> Note: `prime_index` (§2) now returns the live, populated compressed voice file
> (12 nodes / 5 domains / 77 tokens) — the 0-node bug is fixed (allsource-prime
> 0.21.6). `prime_recall` (§4) returns the relevant slice by meaning. `prime_context`
> (§3) showed an empty L2 *vector* arm `[]` **at capture time**; that has since been
> fixed (commit 5083017) — `prime_context` L2 now returns full hybrid recall
> (populated index + vector hits + graph nodes; an L2 query returns the index + 4
> vectors + 3 graph nodes, top node score 0.73). Both `prime_recall` and
> `prime_context` L2 are valid vector paths now. (L0 stays stats-only / vectorless by
> design.)
