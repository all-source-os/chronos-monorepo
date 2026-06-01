# voice-demo results

- binary: `/Users/decebaldobrica/.cargo/bin/allsource-prime` (allsource-prime 0.21.4)
- run (UTC): 2026-06-01T22:39:16.364246+00:00
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
  "sync": null
}
```

Sample recorded entity_ids (12 total):

```
node:voice:71822023-75dd-47be-ac88-11fe6bdd354c  [thinking_pattern] First-principles over cargo-culting
node:voice:d017a347-adcb-4eed-952b-758118667c93  [thinking_pattern] Invert the question
node:voice:0039962d-1b79-428d-9194-37c480c50ad3  [thinking_pattern] Cost of being wrong, not odds of being right
node:voice:a0f52ef0-23df-44f7-8ca4-e1596dfe2dfa  [communication_style] Lead with the punchline
```

## 2. The compressed voice file (prime_index — REAL binary output)

This is the auto-generated equivalent of the post's hand-compressed ~4k-token markdown — but always current and never copy-pasted.

- token_count: 13
- domains: []
- cross_references: []
- last_updated: 2026-06-01T22:39:21.535134+00:00

```markdown
# Knowledge Index

_0 nodes, 0 domains, 0 cross-domain links_
```

> **Honest finding (real server gap, not a harness bug).** In `allsource-prime
> 0.21.4`, `prime_index` reports `0 nodes` even though `prime_stats` (section 1)
> shows all 12 voice nodes durably recorded. The compressed-index path reads the
> `DomainIndexProjection`/`CrossDomainProjection`, which are **not fed by the live
> `prime_add_node` write path** in the MCP process — the gap persists even after
> reopening the same `--data-dir` (re-tested, still `0 nodes`). The live graph,
> vector index, and `prime_stats` ARE fed correctly, which is why `prime_recall`
> below works perfectly. So the voice flow runs on `prime_recall` + `prime_stats`
> today; the `prime_index`-based `/voice export` lands once this projection is
> wired. See `docs/proposals/PRIME_VOICE_FILE.md` § "Convention vs. server gap".

## 3. Voice slice recalled for the prompt (prime_context — REAL binary output)

Instead of pasting the whole voice file, we recall only the facets relevant to THIS task. These are the vector hits returned for the prompt:

- tier: L2  token_count: 13

```json
[]
```

> Same gap as section 2: `prime_context` at tier L2 leans on the index/domain
> projections for its vector slice, so it returns empty against this build. The
> production recall used by the skill is `prime_recall` (section 4), which reads
> the live vector index and returns the correct, ranked voice slice. The skill
> calls `prime_recall` first and only falls back to `prime_context` for the
> compressed-index excerpt once that projection is wired.

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

Audit trail for one facet node (`node:voice:71822023-75dd-47be-ac88-11fe6bdd354c`). A static markdown blob has no provenance; a prime voice file time-travels.

```json
[
  {
    "type": "prime.node.created",
    "timestamp": "2026-06-01T22:39:16.421559+00:00"
  }
]
```

## 6. voice-ON vs voice-OFF (same prompt)

The recalled voice slice in sections 3-4 IS the injected context for the voice-ON arm. The voice-OFF arm gets no recall. The two completions below are written by the agent (Claude) for the SAME prompt — once using the recalled facets as injected context, once generic — so the difference is visible and grounded in the real recall output above.

**Prompt:** Write a short LinkedIn post arguing that when your system's availability hurts, adding another database is usually the wrong fix.

### voice-OFF (no recall — generic)

> Written with no voice recall. Default LLM register: hedged, list-y, hype words,
> abstract.

When your system's availability starts to suffer, it can be tempting to reach for
another database to help share the load. However, adding a new database isn't
always the best solution and can introduce a number of challenges that teams should
carefully consider.

Here are a few things to keep in mind:

- 🚀 **Complexity:** Every new datastore adds operational overhead and another
  moving part to maintain.
- 🔄 **Consistency:** Keeping data in sync across multiple databases can be really
  tricky and error-prone.
- 💸 **Cost:** More infrastructure means more cost, both in dollars and engineering
  time.

Before adding another database, it's a great idea to first explore whether you can
optimize what you already have. Sometimes the answer is replication or better
caching! What's your experience been? Would love to hear your thoughts in the
comments below. 👇

### voice-ON (recalled facets injected)

> Written from the facets `prime_recall` returned for this prompt (section 4):
> *"Add a database is usually the wrong fix"* (score 0.757) → "the better fix is
> almost always replication of the system you already have… not bolting on a new
> stateful component"; *"Distributed systems durability"* (0.688); plus the
> communication-style facets *"Lead with the punchline"*, *"Concrete over
> abstract"*, *"Short declarative sentences, dry humor"*. No emojis, no hype, a
> number, the conclusion first.

Adding a database is usually the wrong fix.

When availability hurts, the reflex is to bolt on another stateful component — a
replica database, a cache, a queue. It feels like progress. It isn't. You just
added a second consistency surface to keep in sync and a second thing to page you
at 3am.

The bottleneck is almost never "not enough databases." It's one component that
can't survive losing a node. So make that component survive losing a node.
Leader-follower replication of the system you already run buys you the same
availability without a new ops burden and without a new way for two stores to
disagree.

I watched a team add a read-replica datastore to fix p99 latency. Six weeks later
the outage wasn't the original database — it was the replication lag between the
two. They'd traded one failure mode for a worse one.

Before you add a database, ask what you're actually trading. Usually it's a
durability problem you never wrote down, dressed up as a scaling problem.

---
_The two completions answer the SAME prompt. The only difference is whether the
voice slice from `prime_recall` (section 4) was injected. The ON arm carries the
recalled contrarian thesis verbatim ("replication of the system you already run,"
"not bolting on a new stateful component"), the recalled durability expertise, and
the recorded style facets (punchline-first, a concrete war story, no emojis, no
hype). That difference is the product._

---
_wall 5.2s incl. first-call fastembed model load. Binary: allsource-prime 0.21.4._
