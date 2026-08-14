# GEO/AEO content experiment — evidence before expansion

Date: 2026-08-14
Status: content remediation implemented; live multi-engine measurement blocked

## Goal

Increase probability that search and answer engines describe AllSource with
correct product category, architecture, pricing, tool counts, and benchmark
scope. Avoid scaled answer pages until canonical facts stop contradicting one
another.

## Hypothesis

Answer engines will produce more accurate, quotable answers when:

1. visible copy gives a direct answer before marketing detail;
2. machine-readable facts match visible content and checkout data;
3. benchmark claims name measured path and caveats;
4. product boundaries and poor-fit cases are explicit;
5. old indexed articles stop repeating retired facts.

## Evidence used

- Live public-page extraction found retired dollar prices, inconsistent MCP
  counts, absolute durability language, and a sub-microsecond claim alongside
  the 11.9µs reference number.
- Repository inspection established Core as WAL + Parquet persistence with a
  concurrent in-memory read path; PostgreSQL is operational metadata only.
- Registry inspection established event-store MCP counts of 45 read-only, 55
  default, 64 with control-plane access, and 73 with system administration.
- Prime registry inspection established 19 `prime_*` memory tools and 27 tools
  when optional inbox and hound modules are included.
- `GET /api/v1/billing/catalog` returned GBP prices of £18.99, £78.99, and
  £298.99 per month; live catalogue output remains authoritative.
- Published benchmark material supports 469K events/sec for the batch-ingest
  reference and 11.9µs p99 for Core indexed reads. It does not establish
  11.9µs end-to-end vector, graph, or hybrid recall.

External guidance used:

- [Google Search guidance for AI features](https://developers.google.com/search/docs/fundamentals/ai-optimization-guide)
- [Bing AI Performance guidance](https://blogs.bing.com/webmaster/February-2026/Introducing-AI-Performance-in-Bing-Webmaster-Tools-Public-Preview)
- [Perplexity crawler documentation](https://docs.perplexity.ai/docs/resources/perplexity-crawlers)

## Iterations shipped in this pass

### 1. Canonical machine truth

- Updated `llms.txt` with current GBP prices, exact tool-count variants,
  storage architecture, and benchmark scope.
- Made paid `SoftwareApplication` offers derive only from live billing-catalog
  output. When catalogue data is unavailable, structured offers are omitted
  instead of guessed from fallbacks.
- Added tests that reject retired prices, old MCP counts, absolute memory
  language, and sub-microsecond claims on canonical answer surfaces.

### 2. Answer-first pages

- Added visible quick answers and matching FAQ JSON-LD to the event-sourcing
  pillar page.
- Explained when event-sourced agent memory is a poor fit.
- Replaced unsourced competitor matrices with Core/Prime/hosted fit guidance.
- Separated Core indexed-read performance from Prime hybrid-recall performance.

### 3. Product and claim boundaries

- Distinguished local Prime from stateless hosted Prime over Core.
- Reframed financial-services compliance copy as supporting evidence, not
  automatic compliance.
- Rebuilt the quant page around shipped event and analytics primitives; removed
  fictional performance, probability, roadmap, and natural-language results.
- Corrected old articles carrying retired prices, tool counts, licensing, or
  absolute durability claims and added `updatedAt` dates.

## Measurement status

The Rust GEO harness completed an offline dry run. Its outputs are synthetic
fixtures and do **not** show a score improvement.

A valid before/after layer-3 sweep still requires provider credentials for
OpenAI, Anthropic, Gemini, and Perplexity plus an AllSource API key. Generated
baseline and remediation files must come from that live run; do not fill them
by hand.

## Remaining blocker discovered

Latest published event-store MCP connector v0.22.0 cannot attach the hosted
gateway Authorization header. Fix exists on main for v0.23.0. Stable hosted-MCP
instructions remain unreliable until that release is published.
