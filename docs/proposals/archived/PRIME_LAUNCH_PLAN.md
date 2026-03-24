# Prime Launch Plan — Marketing, Demos, and Distribution

> **Status**: Active
> **Date**: 2026-03-21
> **Goal**: Get AllSource Prime + Recall in front of developers building AI agents

---

## Priority 1: Publish to crates.io + Claude Desktop config

- [ ] Publish `allsource-prime` binary crate to crates.io (`cargo publish`)
- [ ] Add Claude Desktop MCP config snippet to docs
- [ ] Update GitHub README with Prime section + install command
- [ ] Verify `cargo install allsource-prime` works end-to-end

## Priority 2: Deploy HTTP server on Fly.io + dashboard demo

- [ ] Create `fly.toml` for `allsource-prime --mode http`
- [ ] Deploy to Fly.io on `allsource-prime.fly.dev`
- [ ] Build dashboard demo component at `/dashboard/demo/prime`
  - Interactive graph builder (add nodes, draw edges)
  - Live compressed index preview
  - "How does X relate to Y?" query box with Recall results
- [ ] Pre-seed with revenue/engineering/product dataset

## Priority 3: Blog post — zer0dex comparison

- [ ] Publish `docs/articles/zer0dex-comparison.md` as blog post at `/blog/zer0dex-vs-allsource`
- [ ] Add Open Graph images and meta tags
- [ ] Cross-post to dev.to and Hacker News

## Priority 4: X launch thread

- [ ] Publish the 10-tweet thread from `docs/articles/zer0dex-x-thread.md`
- [ ] Record 60-second demo video of Claude Desktop using Prime MCP tools
- [ ] Pin thread + link to blog post

## Priority 5: Solutions page — `/solutions/agent-memory`

- [ ] Hero: "Give your AI agent perfect memory"
- [ ] Problem section: 3-database diagram
- [ ] Comparison table: AllSource vs zer0dex vs Mem0 vs Letta
- [ ] Link to interactive demo
- [ ] CTA: `cargo install allsource-prime`

## Priority 6: Docs section — `/docs/prime`

- [ ] Quickstart (3 examples rendered as code blocks with output)
- [ ] API Reference (from cargo doc)
- [ ] MCP Setup guide
- [ ] Concepts: compressed index, cross-domain reasoning, temporal queries

## Priority 7: WASM playground (future)

- [ ] Compile `allsource-core` with `prime` feature to WASM
- [ ] Browser-based graph builder + compressed index generator
- [ ] No backend needed, shareable via URL

## Priority 8: Terminal recordings

- [ ] Record asciinema for `prime_graph`, `prime_vectors`, `prime_recall` examples
- [ ] Embed in docs pages

## Priority 9: Benchmark publication

- [ ] Run LoCoMo + LongMemEval benchmarks with real embeddings (fastembed)
- [ ] Publish results as blog post
- [ ] X thread with benchmark data
