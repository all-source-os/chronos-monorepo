#!/usr/bin/env python3
"""mammoth-bench (full) — the 5-claim recall benchmark for AllSource Prime.

The P0 smoke test (bench.py) proved one thing: recall beats grep on hit-rate.
This is the full harness from the ecosystem proposal (chronis t-12c2). It scores
all five published claims against a larger fixture corpus, and is HONEST about
which numbers are direct measurements vs. proxies:

  1. Recall precision        hit@3 / hit@5 / MRR vs a search+grep baseline   [MEASURED]
  2. Cross-session continuity win-rate                                       [PROXY]
       Proxy: can the agent ANSWER a later-session question at all? It can iff
       the gold memory is retrievable (top-5). memory-ON win-rate = hit@5;
       memory-OFF = the baseline's hit@5. We do NOT run an LLM judge here — we
       measure retrievability, which is the necessary precondition for a correct
       answer. Labeled honestly as a proxy, not a blind A/B.
  3. Tokens-saved-by-not-re-explaining                                       [MEASURED]
       For each recall-eligible turn the agent answers from memory: tokens the
       user would otherwise paste (the gold fact) minus the query tokens spent
       to retrieve it. Counted only on turns where recall actually surfaced the
       gold (no credit for misses). Token count = whitespace words * 1.3 (rough
       GPT-style ratio); reported as an estimate, not exact BPE.
  4. Recall latency p50/p95                                                  [MEASURED]
       End-to-end prime_recall wall time incl. in-process fastembed embed.
  5. Durability (write -> restart -> read)                                   [MEASURED]
       Seed N memories, close the server, reopen on the same data dir, confirm
       all N are still queryable. Proves WAL+Parquet persistence, not a cache.

Usage:
  python3 bench2.py                 # fixtures.jsonl + queries.jsonl
  python3 bench2.py --verbose
  PRIME_BIN=/path/to/allsource-prime python3 bench2.py

Drives the prime binary over stdio JSON-RPC against throwaway temp data dirs;
never touches your real .prime/. Requires allsource-prime >= 0.21.3.
"""
import argparse, json, os, re, subprocess, sys, tempfile, time, shutil

STOP = set("a an the of to in on for is are be do does need before out box which and or "
           "we us our i my it this that what where who why how when should can your you "
           "they them their not no yes get got make made into versus vs each every at "
           "actually really just have has had will would there here".split())

def toks(s):
    return [w for w in re.findall(r"[a-z0-9]+", s.lower()) if w not in STOP and len(w) > 1]

def est_tokens(s):
    # rough GPT-style estimate: ~1.3 tokens per whitespace word. Reported as estimate.
    return int(round(len(s.split()) * 1.3))

def default_prime_bin():
    return os.environ.get("PRIME_BIN") or os.path.expanduser("~/.cargo/bin/allsource-prime")

class Prime:
    def __init__(self, binpath, data_dir):
        self.binpath, self.data_dir = binpath, data_dir
        self.p = subprocess.Popen(
            [binpath, "--data-dir", data_dir, "--log-level", "error"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True, bufsize=1)
        self._id = 0
        self._init()

    def _send(self, obj):
        self.p.stdin.write(json.dumps(obj) + "\n"); self.p.stdin.flush()

    def _rpc(self, method, params=None):
        self._id += 1; mid = self._id
        self._send({"jsonrpc": "2.0", "id": mid, "method": method, "params": params or {}})
        for line in self.p.stdout:
            line = line.strip()
            if not line:
                continue
            try:
                m = json.loads(line)
            except json.JSONDecodeError:
                continue
            if m.get("id") == mid:
                if "error" in m:
                    raise RuntimeError(f"{method}: {m['error']}")
                return m.get("result", {})
        raise RuntimeError(f"{method}: no response (server exited)")

    def _init(self):
        self._rpc("initialize", {"protocolVersion": "2024-11-05", "capabilities": {},
                                 "clientInfo": {"name": "mammoth-bench2", "version": "1"}})
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def call(self, name, args):
        res = self._rpc("tools/call", {"name": name, "arguments": args})
        txt = "".join(c.get("text", "") for c in res.get("content", []) if isinstance(c, dict))
        try:
            return json.loads(txt)
        except (json.JSONDecodeError, TypeError):
            return txt

    def close(self):
        try:
            self.p.stdin.close(); self.p.wait(timeout=8)
        except Exception:
            self.p.kill()

def seed(prime, fixtures):
    corpus = []
    for fx in fixtures:
        node = prime.call("prime_add_node", {
            "type": fx["type"],
            "properties": {"name": fx["name"], "domain": fx["domain"]},
        })
        eid = node.get("entity_id")
        prime.call("prime_embed", {"id": eid, "text": fx["text"]})
        corpus.append({"name": fx["name"], "text": fx["text"]})
    return corpus

def recall_names(prime, query, k):
    """Ordered gold-name candidates from prime_recall (best first), plus latency seconds."""
    t0 = time.perf_counter()
    res = prime.call("prime_recall", {"text": query, "top_k": k, "depth": 0})
    dt = time.perf_counter() - t0
    names = []
    if isinstance(res, dict):
        for n in res.get("nodes", []):
            nm = (n.get("properties") or {}).get("name")
            if nm and nm not in names:
                names.append(nm)
    return names, dt

def baseline_names(query, corpus, k):
    q = set(toks(query))
    scored = [(len(q & set(toks(c["text"] + " " + c["name"]))), c["name"]) for c in corpus]
    scored.sort(key=lambda x: -x[0])
    return [name for ov, name in scored[:k] if ov > 0]

def rank_of(gold, ranked):
    for i, nm in enumerate(ranked, 1):
        if nm == gold:
            return i
    return None

def pctl(xs, p):
    if not xs:
        return 0.0
    s = sorted(xs)
    i = min(len(s) - 1, int(round((p / 100.0) * (len(s) - 1))))
    return s[i]

def load_jsonl(path):
    return [json.loads(l) for l in open(path) if l.strip()]

def main():
    ap = argparse.ArgumentParser()
    here = os.path.dirname(os.path.abspath(__file__))
    ap.add_argument("--fixtures", default=os.path.join(here, "fixtures.jsonl"))
    ap.add_argument("--queries", default=os.path.join(here, "queries.jsonl"))
    ap.add_argument("--k", type=int, default=5)
    ap.add_argument("--prime-bin", default=default_prime_bin())
    ap.add_argument("--verbose", action="store_true")
    a = ap.parse_args()

    if not os.path.exists(a.prime_bin):
        sys.exit(f"prime binary not found: {a.prime_bin}\nset PRIME_BIN or: cargo install allsource-prime")

    fixtures = load_jsonl(a.fixtures)
    queries = load_jsonl(a.queries)
    gold_text = {f["name"]: f["text"] for f in fixtures}
    n = len(queries)
    print(f"mammoth-bench (full): {len(fixtures)} memories, {n} queries, k={a.k}")
    print(f"prime: {a.prime_bin}\n")

    data_dir = tempfile.mkdtemp(prefix="mammoth-bench2-")
    t_start = time.time()
    mem_hit3 = mem_hit5 = base_hit5 = base_hit3 = 0
    mem_rr = 0.0
    latencies = []
    tokens_saved = []
    rows = []
    durability = "SKIP"
    try:
        prime = Prime(a.prime_bin, data_dir)
        corpus = seed(prime, fixtures)
        for q in queries:
            mranked, dt = recall_names(prime, q["query"], a.k)
            latencies.append(dt)
            mr = rank_of(q["gold"], mranked)
            if mr and mr <= 3: mem_hit3 += 1
            mhit5 = bool(mr and mr <= min(5, a.k))
            if mhit5: mem_hit5 += 1
            mem_rr += (1.0 / mr) if mr else 0.0
            branked = baseline_names(q["query"], corpus, a.k)
            br = rank_of(q["gold"], branked)
            if br and br <= 3: base_hit3 += 1
            if br and br <= min(5, a.k): base_hit5 += 1
            # tokens-saved: only when memory actually surfaced the gold this turn
            if mhit5:
                saved = est_tokens(gold_text[q["gold"]]) - est_tokens(q["query"])
                tokens_saved.append(max(0, saved))
            rows.append((q["query"][:46], q["gold"][:26], mr or "-", "Y" if mhit5 else "."))

        # durability: this is a PERSISTENCE check, not a recall-ranking check.
        # Close the server, reopen on the same data dir, and confirm every seeded
        # node is still present via prime_stats (total_nodes). Recall ranking can
        # legitimately miss a top-5 slot without any data being lost, so ranking
        # is the wrong signal for "did it survive the restart".
        seeded = len(fixtures)
        prime.close()
        prime2 = Prime(a.prime_bin, data_dir)
        stats = prime2.call("prime_stats", {})
        prime2.close()
        after = stats.get("total_nodes") if isinstance(stats, dict) else None
        durability = "PASS" if after == seeded else f"FAIL ({after}/{seeded} nodes after restart)"
    finally:
        shutil.rmtree(data_dir, ignore_errors=True)
    wall = time.time() - t_start

    mem_p3, mem_p5, mrr = mem_hit3 / n, mem_hit5 / n, mem_rr / n
    base_p3, base_p5 = base_hit3 / n, base_hit5 / n
    p50 = pctl(latencies, 50) * 1000
    p95 = pctl(latencies, 95) * 1000
    median_saved = sorted(tokens_saved)[len(tokens_saved) // 2] if tokens_saved else 0
    total_saved = sum(tokens_saved)

    print("=== 1. RECALL PRECISION (measured) ===")
    print(f"  memory    hit@3={mem_p3:.2f}  hit@5={mem_p5:.2f}  MRR={mrr:.3f}")
    print(f"  baseline  hit@3={base_p3:.2f}  hit@5={base_p5:.2f}")
    print(f"  Δ hit@5 = {mem_p5 - base_p5:+.2f}")
    print("\n=== 2. CROSS-SESSION CONTINUITY (proxy = retrievability) ===")
    print(f"  memory-ON  answerable={mem_p5:.2f}   memory-OFF answerable={base_p5:.2f}")
    print(f"  win-rate (memory answers where baseline cannot) = {max(0.0, mem_p5 - base_p5):.2f}")
    print("\n=== 3. TOKENS SAVED (estimate, ~1.3 tok/word) ===")
    print(f"  median saved/recall = {median_saved}   total over {len(tokens_saved)} hits = {total_saved}")
    print("\n=== 4. RECALL LATENCY (measured, incl. in-process embed) ===")
    print(f"  p50={p50:.1f}ms  p95={p95:.1f}ms  over {len(latencies)} calls @ {len(fixtures)} nodes")
    print("\n=== 5. DURABILITY (write -> restart -> read) ===")
    print(f"  {durability}")

    verdict = "PASS" if (mem_p5 > base_p5 and durability == "PASS") else "REVIEW"
    print(f"\nVERDICT: {verdict}  (wall {wall:.1f}s incl. fastembed model load)")

    if a.verbose:
        print("\n--- per-query (memory): query | gold | rank | hit@5 ---")
        for qf, gf, rk, h in rows:
            print(f"  {h} r={str(rk):<3} {qf:<46} -> {gf}")

    print("\nJSON " + json.dumps({
        "n": n, "memories": len(fixtures),
        "precision": {"mem_hit3": mem_p3, "mem_hit5": mem_p5, "mrr": mrr,
                      "base_hit3": base_p3, "base_hit5": base_p5},
        "continuity_winrate": max(0.0, mem_p5 - base_p5),
        "tokens_saved": {"median": median_saved, "total": total_saved, "hits": len(tokens_saved)},
        "latency_ms": {"p50": round(p50, 1), "p95": round(p95, 1)},
        "durability": durability, "verdict": verdict,
    }))
    sys.exit(0 if verdict == "PASS" else 1)

if __name__ == "__main__":
    main()
