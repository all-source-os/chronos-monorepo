#!/usr/bin/env python3
"""voice-demo — prove the prime-backed "voice file" works against the REAL binary.

The viral pattern (Ruben Hassid): run yourself through a ~100-question interview,
compress the transcript into a ~4k-token markdown "voice file", paste it into any
AI tool so output sounds like you. Weakness: that markdown is a dead blob — it
goes stale, it's one undifferentiated lump, it can't be queried by relevance, no
history.

This harness shows the prime-backed alternative is real, using ONLY existing
prime_* MCP tools (no server changes):

  1. Seed ~12 voice facets as embedded `voice` nodes (the interview, recorded
     live — no separate "compress the transcript" step).
  2. prime_index    -> the compressed, always-current voice file (replaces the
                       hand-compressed 4k-token markdown).
  3. prime_context  -> recall ONLY the relevant voice slice for a writing task,
                       instead of pasting the whole file every time.
  4. prime_history  -> the voice has provenance / can time-travel (a dead blob
                       can't).

Then it prints, for a single writing prompt, the recalled voice slice (voice-ON
context) next to the empty/no-recall case (voice-OFF), so the agent can write the
same prompt twice — once in voice, once generic — and the difference is grounded
in real recall output.

Drives `allsource-prime` over stdio JSON-RPC against a throwaway temp --data-dir,
exactly like tooling/mammoth-bench/bench.py. Requires allsource-prime >= 0.21.3.

Usage:
  python3 voice_demo.py                       # uses ./facets.jsonl
  PRIME_BIN=/path/to/allsource-prime python3 voice_demo.py
  python3 voice_demo.py --prompt "Write a LinkedIn post about why we don't add a database."
"""
import argparse, json, os, subprocess, sys, tempfile, time, shutil, datetime

# The writing task both arms answer. Chosen to land on multiple facets
# (contrarian "add a database is wrong" + expertise "durability" + style).
DEFAULT_PROMPT = ("Write a short LinkedIn post arguing that when your system's "
                  "availability hurts, adding another database is usually the wrong fix.")


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
                                 "clientInfo": {"name": "voice-demo", "version": "1"}})
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


def load_jsonl(path):
    out = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                out.append(json.loads(line))
    return out


def seed_voice(prime, facets):
    """Record each interview answer as an embedded `voice` node, cross-linked
    by domain. This is the interview recorded live — no separate compression step.
    Returns list of (entity_id, facet_dict)."""
    created = []
    for fx in facets:
        node = prime.call("prime_add_node", {
            "type": "voice",
            "properties": {
                "name": fx["name"],
                "domain": fx["domain"],
                "facet": fx["facet"],
                "statement": fx["text"],
            },
        })
        eid = node.get("entity_id")
        prime.call("prime_embed", {"id": eid, "text": fx["text"],
                                   "metadata": {"facet": fx["facet"], "domain": fx["domain"]}})
        created.append((eid, fx))
    # Cross-link facets within the same facet group so neighbors/graph expansion
    # has structure (e.g. two contrarian takes relate to each other).
    by_facet = {}
    for eid, fx in created:
        by_facet.setdefault(fx["facet"], []).append(eid)
    edges = 0
    for eids in by_facet.values():
        for a, b in zip(eids, eids[1:]):
            prime.call("prime_add_edge", {"source": a, "target": b, "relation": "relates_to"})
            edges += 1
    return created, edges


def main():
    ap = argparse.ArgumentParser()
    here = os.path.dirname(os.path.abspath(__file__))
    ap.add_argument("--facets", default=os.path.join(here, "facets.jsonl"))
    ap.add_argument("--prompt", default=DEFAULT_PROMPT)
    ap.add_argument("--prime-bin", default=default_prime_bin())
    ap.add_argument("--out", default=os.path.join(here, "RESULTS.md"))
    ap.add_argument("--top-k", type=int, default=5)
    a = ap.parse_args()

    if not os.path.exists(a.prime_bin):
        sys.exit(f"prime binary not found: {a.prime_bin}\n"
                 f"set PRIME_BIN or run: cargo install allsource-prime")

    facets = load_jsonl(a.facets)
    version = subprocess.run([a.prime_bin, "--version"], capture_output=True, text=True).stdout.strip()
    ts = datetime.datetime.now(datetime.timezone.utc).isoformat()

    log = []  # lines captured into RESULTS.md
    def cap(s=""):
        print(s)
        log.append(s)

    cap(f"# voice-demo results")
    cap()
    cap(f"- binary: `{a.prime_bin}` ({version})")
    cap(f"- run (UTC): {ts}")
    cap(f"- facets seeded: {len(facets)}")
    cap(f"- prompt: {a.prompt!r}")
    cap()
    cap("All output below is verbatim from the real `allsource-prime` binary over "
        "stdio MCP, against a throwaway temp `--data-dir`. Nothing here is hand-written.")
    cap()

    data_dir = tempfile.mkdtemp(prefix="voice-demo-")
    t0 = time.time()
    try:
        prime = Prime(a.prime_bin, data_dir)

        # 1. Record the interview answers as embedded voice nodes.
        created, edges = seed_voice(prime, facets)

        # 2. prime_stats — prove the facets are durably recorded.
        stats = prime.call("prime_stats", {})
        cap("## 1. Facets recorded (prime_stats — REAL binary output)")
        cap()
        cap("```json")
        cap(json.dumps({
            "total_nodes": stats.get("total_nodes"),
            "total_edges": stats.get("total_edges"),
            "event_count": stats.get("event_count"),
            "nodes_by_type": stats.get("nodes_by_type"),
            "edges_by_relation": stats.get("edges_by_relation"),
            "sync": stats.get("sync", {}).get("enabled"),
        }, indent=2))
        cap("```")
        cap()
        cap(f"Sample recorded entity_ids ({len(created)} total):")
        cap()
        cap("```")
        for eid, fx in created[:4]:
            cap(f"{eid}  [{fx['facet']}] {fx['name']}")
        cap("```")
        cap()

        # 3. prime_index — the compressed, always-current voice file.
        idx = prime.call("prime_index", {})
        cap("## 2. The compressed voice file (prime_index — REAL binary output)")
        cap()
        cap("This is the auto-generated equivalent of the post's hand-compressed "
            "~4k-token markdown — but always current and never copy-pasted.")
        cap()
        cap(f"- token_count: {idx.get('token_count')}")
        cap(f"- domains: {idx.get('domains')}")
        cap(f"- cross_references: {idx.get('cross_references')}")
        cap(f"- last_updated: {idx.get('last_updated')}")
        cap()
        cap("```markdown")
        cap((idx.get("index") or "").rstrip())
        cap("```")
        cap()

        # 4. prime_context — recall the relevant voice slice for THIS prompt.
        ctx = prime.call("prime_context", {"query": a.prompt, "top_k": a.top_k, "tier": "L2"})
        cap("## 3. Voice slice recalled for the prompt (prime_context — REAL binary output)")
        cap()
        cap("Instead of pasting the whole voice file, we recall only the facets "
            "relevant to THIS task. These are the vector hits returned for the prompt:")
        cap()
        cap(f"- tier: {ctx.get('tier')}  token_count: {ctx.get('token_count')}")
        cap()
        cap("```json")
        vecs = ctx.get("vectors") or []
        cap(json.dumps([
            {"text": v.get("text"), "score": round(v.get("score", 0), 4)}
            for v in vecs
        ], indent=2))
        cap("```")
        cap()

        # 5. prime_recall (depth=0, vector-only) — a tighter slice, same prompt.
        rec = prime.call("prime_recall", {"text": a.prompt, "top_k": a.top_k, "depth": 1})
        cap("## 4. prime_recall for the prompt (REAL binary output)")
        cap()
        cap("```json")
        cap(json.dumps({
            "nodes": [
                {"name": (n.get("properties") or {}).get("name"),
                 "facet": (n.get("properties") or {}).get("facet"),
                 "score": round(n.get("score", 0), 4), "depth": n.get("depth")}
                for n in (rec.get("nodes") or [])
            ],
            "vectors": [
                {"text_head": (v.get("text") or "")[:70], "score": round(v.get("score", 0), 4)}
                for v in (rec.get("vectors") or [])
            ],
            "edges": rec.get("edges"),
        }, indent=2))
        cap("```")
        cap()

        # 6. prime_history — the voice has provenance (a dead blob doesn't).
        sample_eid = created[0][0]
        hist = prime.call("prime_history", {"entity_id": sample_eid})
        cap("## 5. The voice has history (prime_history — REAL binary output)")
        cap()
        cap(f"Audit trail for one facet node (`{sample_eid}`). A static markdown blob "
            "has no provenance; a prime voice file time-travels.")
        cap()
        cap("```json")
        cap(json.dumps([
            {"type": e.get("type"), "timestamp": e.get("timestamp")}
            for e in (hist.get("events") or [])
        ], indent=2))
        cap("```")
        cap()

        prime.close()
    finally:
        shutil.rmtree(data_dir, ignore_errors=True)
    dt = time.time() - t0

    # 7. The voice-ON vs voice-OFF comparison scaffold. The recalled slice above
    # is the injected context for the ON arm; the OFF arm gets nothing. The agent
    # (Claude) writes both completions and pastes them verbatim under these headers.
    cap("## 6. voice-ON vs voice-OFF (same prompt)")
    cap()
    cap("The recalled voice slice in sections 3-4 IS the injected context for the "
        "voice-ON arm. The voice-OFF arm gets no recall. The two completions below "
        "are written by the agent (Claude) for the SAME prompt — once using the "
        "recalled facets as injected context, once generic — so the difference is "
        "visible and grounded in the real recall output above.")
    cap()
    cap(f"**Prompt:** {a.prompt}")
    cap()
    cap("### voice-OFF (no recall — generic)")
    cap()
    cap("<!-- AGENT: paste the generic completion here -->")
    cap()
    cap("### voice-ON (recalled facets injected)")
    cap()
    cap("<!-- AGENT: paste the in-voice completion here, written from the recalled facets -->")
    cap()
    cap("---")
    cap(f"_wall {dt:.1f}s incl. first-call fastembed model load. Binary: {version}._")

    with open(a.out, "w") as f:
        f.write("\n".join(log) + "\n")
    print(f"\n[voice-demo] wrote {a.out}", file=sys.stderr)


if __name__ == "__main__":
    main()
