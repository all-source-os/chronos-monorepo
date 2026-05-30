# mammoth-bench

Precision@k harness for AllSource Prime recall — the P0 GO/KILL gate for the
**mammoth** durable-agent-memory ecosystem (see
`docs/proposals/ALLSOURCE_AGENT_MEMORY_ECOSYSTEM.md`, chronis bead `t-f6bf`).

## What it proves

The kill-criterion for mammoth: *does semantic recall beat a keyword search+grep
baseline at surfacing the right prior memory for a **differently-worded**
query?* If memory can't beat grep, the magic moment doesn't exist and the
project stops.

Two arms over the same seeded store and the same queries:

- **memory** — `prime_recall(text=query)` (in-process fastembed `AllMiniLML6V2`
  → HNSW vectors + graph).
- **baseline** — keyword token-overlap grep over the same stored memory texts.

Each query has a single labeled gold memory, so "Precision@5" is read honestly as
hit-rate (gold appears in top-k). Also reports MRR.

## Two harnesses

- **`bench.py`** — the P0 smoke gate: precision@k only, fast. One claim, one table.
- **`bench2.py`** — the full 5-claim harness (bead `t-12c2`): precision, a
  cross-session continuity proxy, tokens-saved, recall latency p50/p95, and a
  write→restart→read durability check. Honest about which numbers are measured
  vs. proxied (see the docstring).

## Run

```bash
cd tooling/mammoth-bench
python3 bench.py             # P0 smoke: precision only
python3 bench2.py            # full 5-claim benchmark
python3 bench2.py --verbose  # + per-query ranks
```

Needs `allsource-prime >= 0.21.3` on `~/.cargo/bin` (or set `PRIME_BIN`). Both
harnesses drive the binary over stdio JSON-RPC against a throwaway temp
`--data-dir`, so they never touch your real `.prime/` store. Exit non-zero on
failure (so they can gate CI).

## Files

- `fixtures.jsonl` — memories to seed (`type`, `domain`, `name`, `text`). Each is
  a real AllSource/mammoth decision or gotcha, so recall quality reflects this
  project's actual knowledge.
- `queries.jsonl` — `query` (worded unlike the stored text, on purpose) + `gold`
  (the fixture `name` it should recall).
- `bench.py` — P0 smoke harness (precision only).
- `bench2.py` — full 5-claim harness.
- `RESULTS.txt` / `RESULTS-full.txt` — last captured runs.

## Results

### P0 smoke — `bench.py` (2026-05-30, 18 memories / 18 queries, k=5)

| metric | memory | baseline | Δ |
|--------|--------|----------|------|
| hit@3  | 0.89   | 0.78     | +0.11 |
| hit@5  | 0.94   | 0.78     | +0.17 |
| MRR    | 0.826  | 0.722    | +0.104 |

**PASS** — memory beats baseline. Single miss: an under-specified query
("what are we calling this project?") where token overlap is near-zero.

### Full 5-claim — `bench2.py` (2026-05-30, 60 memories / 60 queries, k=5)

| # | metric | result | kind |
|---|--------|--------|------|
| 1 | Recall hit@5 | **0.90** (baseline 0.83, Δ +0.07) | measured |
| 1 | Recall hit@3 / MRR | 0.87 / 0.783 | measured |
| 2 | Cross-session continuity win-rate | 0.07 | proxy* |
| 3 | Median tokens saved / recall | 19 (986 total over 54 hits) | estimate |
| 4 | Recall latency p50 / p95 | **3.0ms / 3.6ms** | measured |
| 5 | Durability (write→restart→read) | **PASS** | measured |

**VERDICT: PASS.** \*Continuity is a *proxy* — retrievability (can the agent
answer at all), not a blind LLM-judged A/B. Durability checks node count via
`prime_stats` after a real server restart (persistence), not recall ranking.

## Honesty rule (applied)

The precision edge over grep **narrowed from +0.17 (18 memories) to +0.07 (60)**
— expected: a denser corpus gives the keyword baseline more overlap to exploit.
Memory still wins every metric, latency stays sub-4ms, durability holds. Per the
proposal, the softening is **published, not hidden**. The improvement levers
(hybrid keyword+vector, graph expansion via `depth`, a larger embedder) are
deliberately unused here — headroom, not yet spent. Same trust move caveman made
with "output tokens only".
