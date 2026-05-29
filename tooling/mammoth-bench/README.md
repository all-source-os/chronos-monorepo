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

## Run

```bash
cd tooling/mammoth-bench
python3 bench.py            # uses fixtures.jsonl + queries.jsonl
python3 bench.py --verbose  # per-query ranks
```

Needs `allsource-prime >= 0.21.3` on `~/.cargo/bin` (or set `PRIME_BIN`). The
harness drives the binary over stdio JSON-RPC against a throwaway temp
`--data-dir`, so it never touches your real `.prime/` store. Exits non-zero if
memory fails to beat baseline (so it can gate CI).

## Files

- `fixtures.jsonl` — memories to seed (`type`, `domain`, `name`, `text`). Each is
  a real AllSource/mammoth decision or gotcha, so recall quality reflects this
  project's actual knowledge.
- `queries.jsonl` — `query` (worded unlike the stored text, on purpose) + `gold`
  (the fixture `name` it should recall).
- `bench.py` — the harness.
- `RESULTS.txt` — last captured run.

## Baseline result (2026-05-30, 18 memories / 18 queries, k=5)

| metric | memory | baseline | Δ |
|--------|--------|----------|------|
| hit@3  | 0.89   | 0.78     | +0.11 |
| hit@5  | 0.94   | 0.78     | +0.17 |
| MRR    | 0.826  | 0.722    | +0.104 |

**PASS** — memory beats baseline. Single miss: an under-specified query
("what are we calling this project?") where token overlap is near-zero in both
arms.

## Honesty rule

This is the small-corpus smoke version of the P0 gate. The full benchmark
(bead `t-12c2`) must scale to 50–100 multi-session transcripts and add
cross-session continuity win-rate, tokens-saved, and recall latency. If recall
quality drops on the larger corpus, publish it anyway and show the improvement
curve (hybrid keyword+vector, graph `depth`, larger embedder) — same trust move
caveman made with "output tokens only".
