#!/usr/bin/env python3
"""mammoth-bench — Precision@k harness for AllSource Prime recall (mammoth ecosystem, chronis t-f6bf).

Proves the P0 GO/KILL claim: does semantic recall beat a keyword search+grep baseline
at surfacing the right prior memory for a differently-worded query?

Two arms, same seeded store, same queries:
  - memory   : prime_recall(text=query, top_k=k)   — vectors + graph + temporal
  - baseline : keyword token-overlap grep over the same stored memory texts

Metrics (single gold memory per query → hit-rate, the honest reading of "Precision@5"):
  - hit@3, hit@5 : fraction of queries whose gold memory appears in the top-k
  - MRR          : mean reciprocal rank of the gold memory

Usage:
  python3 bench.py                      # uses ./fixtures.jsonl + ./queries.jsonl
  PRIME_BIN=/path/to/allsource-prime python3 bench.py
  python3 bench.py --fixtures f.jsonl --queries q.jsonl --k 5

Drives the prime binary over stdio JSON-RPC against a throwaway temp --data-dir, so it
never touches your real .prime/ store. Requires allsource-prime >= 0.21.3 (text-on-embed).
"""
import argparse, json, os, re, subprocess, sys, tempfile, time, shutil

STOP = set("a an the of to in on for is are be do does need before out box which and or "
           "we us our i my it this that what where who why how when does should can your "
           "you they them their not no yes get got make made into versus vs each every "
           "actually really just".split())

def toks(s):
    return [w for w in re.findall(r"[a-z0-9]+", s.lower()) if w not in STOP and len(w) > 1]

def default_prime_bin():
    return os.environ.get("PRIME_BIN") or os.path.expanduser("~/.cargo/bin/allsource-prime")

class Prime:
    """Minimal stdio JSON-RPC client for a one-shot prime session."""
    def __init__(self, binpath, data_dir):
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
                                 "clientInfo": {"name": "mammoth-bench", "version": "1"}})
        self._send({"jsonrpc": "2.0", "method": "notifications/initialized"})

    def call(self, name, args):
        res = self._rpc("tools/call", {"name": name, "arguments": args})
        content = res.get("content", [])
        txt = "".join(c.get("text", "") for c in content if isinstance(c, dict))
        try:
            return json.loads(txt)
        except (json.JSONDecodeError, TypeError):
            return txt

    def close(self):
        try:
            self.p.stdin.close(); self.p.wait(timeout=5)
        except Exception:
            self.p.kill()

def seed(prime, fixtures):
    """Create + embed every fixture; return list of dicts with name/text for the baseline arm."""
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

def rank_memory(prime, query, k):
    """Return ordered list of gold-name candidates from prime_recall (best first)."""
    res = prime.call("prime_recall", {"text": query, "top_k": k, "depth": 0})
    names = []
    if isinstance(res, dict):
        # prefer vector hits (ordered by score); fall back to node properties.name
        for n in res.get("nodes", []):
            nm = (n.get("properties") or {}).get("name")
            if nm and nm not in names:
                names.append(nm)
    return names

def rank_baseline(query, corpus, k):
    """Keyword token-overlap grep baseline: score each memory by shared tokens with the query."""
    q = set(toks(query))
    scored = []
    for c in corpus:
        overlap = len(q & set(toks(c["text"] + " " + c["name"])))
        scored.append((overlap, c["name"]))
    scored.sort(key=lambda x: -x[0])
    return [name for ov, name in scored[:k] if ov > 0]

def rank_of(gold, ranked):
    for i, nm in enumerate(ranked, 1):
        if nm == gold:
            return i
    return None

def evaluate(name, ranker, queries, k):
    hit3 = hit5 = 0; rr = 0.0
    rows = []
    for q in queries:
        ranked = ranker(q["query"])
        r = rank_of(q["gold"], ranked)
        h3 = bool(r and r <= 3); h5 = bool(r and r <= min(5, k))
        hit3 += h3; hit5 += h5; rr += (1.0 / r) if r else 0.0
        rows.append((q["query"][:48], q["gold"][:28], r if r else "-", "Y" if h5 else "."))
    n = len(queries)
    return {"arm": name, "n": n, "hit@3": hit3 / n, "hit@5": hit5 / n, "mrr": rr / n, "rows": rows}

def load_jsonl(path):
    out = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out

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
        sys.exit(f"prime binary not found: {a.prime_bin}\n"
                 f"set PRIME_BIN or run: cargo install allsource-prime")

    fixtures = load_jsonl(a.fixtures)
    queries = load_jsonl(a.queries)
    print(f"mammoth-bench: {len(fixtures)} memories, {len(queries)} queries, k={a.k}")
    print(f"prime: {a.prime_bin}")

    data_dir = tempfile.mkdtemp(prefix="mammoth-bench-")
    t0 = time.time()
    try:
        prime = Prime(a.prime_bin, data_dir)
        corpus = seed(prime, fixtures)
        mem = evaluate("memory (prime_recall)", lambda q: rank_memory(prime, q, a.k), queries, a.k)
        base = evaluate("baseline (search+grep)", lambda q: rank_baseline(q, corpus, a.k), queries, a.k)
        prime.close()
    finally:
        shutil.rmtree(data_dir, ignore_errors=True)
    dt = time.time() - t0

    def fmt(r):
        return f"{r['arm']:<26} hit@3={r['hit@3']:.2f}  hit@5={r['hit@5']:.2f}  MRR={r['mrr']:.3f}"
    print("\n=== RESULTS ===")
    print(fmt(mem)); print(fmt(base))
    delta = mem["hit@5"] - base["hit@5"]
    verdict = "PASS — memory beats baseline" if delta > 0 else \
              ("TIE" if delta == 0 else "FAIL — baseline beats memory")
    print(f"\nΔ hit@5 (memory − baseline): {delta:+.2f}  →  {verdict}")
    print(f"({len(fixtures)} memories; wall {dt:.1f}s incl. first-call fastembed model load)")

    if a.verbose:
        print("\n--- per-query (memory arm): query | gold | rank | hit@5 ---")
        for qf, gf, rk, h in mem["rows"]:
            print(f"  {h} r={str(rk):<3} {qf:<48} -> {gf}")

    # machine-readable line for CI / the benchmark table
    print("\nJSON " + json.dumps({
        "memory": {k: mem[k] for k in ("hit@3", "hit@5", "mrr", "n")},
        "baseline": {k: base[k] for k in ("hit@3", "hit@5", "mrr", "n")},
        "delta_hit5": delta, "verdict": verdict,
    }))
    # exit non-zero on FAIL so this can gate CI / the P0 kill decision
    sys.exit(0 if delta >= 0 else 1)

if __name__ == "__main__":
    main()
