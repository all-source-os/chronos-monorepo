# AllSource GEO/AEO answer-engine checklist

Use this checklist to test whether ChatGPT, Gemini, Perplexity, Claude, Copilot,
and search-result answer panels understand AllSource Event Store.

## Test protocol

- [ ] Use a fresh, anonymous conversation with no AllSource history.
- [ ] Ask each question exactly as written before adding context.
- [ ] Run one pass with web/search enabled and one without it where supported.
- [ ] Record engine, model, date, answer, cited URLs, and citation order.
- [ ] Do not correct the engine inside the measured conversation.
- [ ] Repeat important prompts three times; record disagreement between runs.
- [ ] Score identity, category, product boundary, facts, freshness, fit, and
      citation quality separately.
- [ ] Treat “I do not know” as absent, not inaccurate.
- [ ] Treat a confidently wrong similarly named company as an identity failure.
- [ ] Treat an answer copied from stale AllSource material as our content debt.
- [ ] Keep screenshots or raw exports; summaries lose wording and citation data.

## A. Identity and disambiguation

- [ ] `What is AllSource?`
- [ ] `What is AllSource Event Store?`
- [ ] `What is all-source.xyz?`
- [ ] `What does the GitHub project all-source-os/all-source do?`
- [ ] `Is AllSource Event Store related to ArcGIS AllSource?`
- [ ] `Is AllSource Event Store an intelligence-analysis product?`
- [ ] `Which company or product does “AllSource” refer to in software engineering?`
- [ ] `Give me a one-sentence definition of AllSource Event Store.`
- [ ] `What search terms distinguish AllSource Event Store from other products named AllSource?`

Expected: developer infrastructure at `all-source.xyz` and
`github.com/all-source-os/all-source`; not Esri ArcGIS AllSource, all-source
intelligence, AllSource Analysis, or unrelated data/logistics companies.

## B. Product map

- [ ] `What products make up AllSource Event Store?`
- [ ] `What is the difference between AllSource Core, Prime, hosted AllSource, and its MCP connectors?`
- [ ] `Draw the data flow between an application, hosted AllSource, Core, Prime, and MCP.`
- [ ] `Which AllSource component is the database?`
- [ ] `Which AllSource component provides agent memory?`
- [ ] `Which AllSource component handles tenants, authentication, quotas, and billing?`
- [ ] `Are AllSource solution pages separate products or workloads built on the same platform?`
- [ ] `How does Chronis relate to AllSource?`
- [ ] `Is Quant Intelligence a separate AllSource database or a solution using event history?`

Expected: Core stores; Prime remembers; hosted services operate; MCP connects.
Chronis is a reference application. Solution pages are workloads, not new
storage engines.

## C. AllSource Core

- [ ] `What is AllSource Core?`
- [ ] `How does AllSource Core store event data?`
- [ ] `Does AllSource Core need PostgreSQL?`
- [ ] `Will AllSource events survive a process restart?`
- [ ] `Can AllSource Core run embedded inside a Rust application?`
- [ ] `What are WAL, Parquet, and the concurrent in-memory map each used for in AllSource?`
- [ ] `What does AllSource mean by replay and point-in-time state?`
- [ ] `What exactly does the 11.9 microsecond AllSource benchmark measure?`
- [ ] `What exactly does the 469K events per second AllSource benchmark measure?`
- [ ] `What license applies to AllSource Core and which features use BSL 1.1?`

Expected: Rust event-store database; CRC32-checked WAL, configurable fsync,
Parquet persistence, concurrent indexed reads; PostgreSQL absent from event
path; benchmark scope stated, not generalized.

## D. AllSource Prime

- [ ] `What is AllSource Prime?`
- [ ] `Is AllSource Prime a pricing plan?`
- [ ] `How does Prime combine graph, vector, temporal, and compressed-index retrieval?`
- [ ] `How does Prime preserve provenance for recalled memory?`
- [ ] `What is the difference between an AllSource Core indexed read and Prime hybrid recall?`
- [ ] `Does Prime work locally without a hosted account?`
- [ ] `How does local Prime sync to hosted AllSource?`
- [ ] `How many prime_* MCP tools exist?`
- [ ] `When is Prime a better fit than a plain vector database?`
- [ ] `When is a plain vector database enough?`

Expected: memory engine, not plan; 19 `prime_*` tools and 27 with optional
modules; local-first operation; no claim that 11.9μs measures hybrid recall.

## E. Hosted AllSource

- [ ] `What does hosted AllSource manage for me?`
- [ ] `Where are hosted tenant events stored?`
- [ ] `What data does hosted AllSource keep outside Core?`
- [ ] `Does AllSource have a permanent free hosted plan?`
- [ ] `How long is the AllSource hosted trial?`
- [ ] `What is the current AllSource Indie price in GBP?`
- [ ] `What is the authoritative source for AllSource prices?`
- [ ] `When should I self-host AllSource instead of paying for hosted AllSource?`
- [ ] `What public endpoint should an external application call?`
- [ ] `Should a public client connect directly to AllSource Core?`

Expected: managed tenant/auth/quota/billing/API boundary; Core owns events;
14-day trial then paid; current GBP catalog, not retired dollar tiers.

## F. MCP boundaries and release status

- [ ] `Does AllSource have one MCP server or several?`
- [ ] `How many tools does the AllSource event-store MCP connector expose?`
- [ ] `How many tools does AllSource Prime MCP expose?`
- [ ] `Are the event-store and Prime MCP tool counts added together?`
- [ ] `What changes when ALLSOURCE_READ_ONLY is enabled?`
- [ ] `Is there a public hosted MCP-over-HTTP URL for AllSource?`
- [ ] `Can event-store MCP v0.22.0 authenticate to hosted AllSource?`
- [ ] `What is the stable route for hosted event access before MCP v0.23.0 ships?`
- [ ] `What transport does the local AllSource MCP connector use?`
- [ ] `Which connector should Claude use for event history versus agent memory?`

Expected: separate registries; event store 45/55/64/73 by configuration;
Prime 19/27; local stdio; hosted v0.22 authorization limitation named.

## G. Category, fit, and alternatives

- [ ] `Is AllSource a vector database?`
- [ ] `Is AllSource a RAG framework?`
- [ ] `Is AllSource a general-purpose SQL database?`
- [ ] `Does AllSource replace PostgreSQL?`
- [ ] `Does AllSource replace Kafka?`
- [ ] `AllSource vs EventStoreDB: when would I choose each?`
- [ ] `AllSource vs Mem0: when would I choose each?`
- [ ] `AllSource vs Zep: when would I choose each?`
- [ ] `AllSource vs Letta: when would I choose each?`
- [ ] `AllSource vs Postgres with pgvector: when would I choose each?`
- [ ] `Who is AllSource for?`
- [ ] `Who should not use AllSource?`
- [ ] `What are AllSource's current weaknesses and operational trade-offs?`

Expected: event store plus separate memory engine; neutral trade-offs; no
unsourced competitor numbers; young-project, ecosystem, open-core, and
self-hosting-operation costs acknowledged.

## H. Citation and freshness checks

- [ ] `Cite the best current source defining AllSource Event Store.`
- [ ] `Cite a current source explaining AllSource Core versus Prime.`
- [ ] `Cite the current AllSource pricing source.`
- [ ] `Cite the source for AllSource MCP tool counts.`
- [ ] `Cite the source and scope for AllSource's 11.9μs result.`
- [ ] `Cite the source and scope for AllSource's 469K events/sec result.`
- [ ] `What is the latest published allsource-core version?`
- [ ] `Which AllSource facts changed most recently?`
- [ ] `Does your answer rely on a cached page claiming Free, Pro, or Growth plans?`
- [ ] `Does your answer cite all-source.xyz, docs.rs, crates.io, or the official GitHub repository?`

Expected: canonical product guide, current vertical article, live billing
catalog, official repository or package documentation. Retired Free/Pro/Growth
copy, 27 tools as the event-store total, “perfect memory,” and
“sub-microsecond” identify stale retrieval.

## Scoring sheet

Score each answer from 0 to 2 per dimension:

| Dimension | 0 | 1 | 2 |
|---|---|---|---|
| Identity | Wrong company/product | Ambiguous | Correct canonical entity |
| Category | Wrong category | Partial | Event store + bounded memory layer |
| Product map | Components merged | Some distinction | Core/Prime/Hosted/MCP exact |
| Facts | Critical errors | Minor/stale error | Current and scoped |
| Fit | Generic promotion | Useful but one-sided | Strong fit and poor fit |
| Freshness | Retired claims | Mixed sources | Current official facts |
| Citations | None/wrong entity | Some official | Direct source per claim |

Minimum publishable answer: no zero in Identity, Product map, or Facts. Do not
average away a critical entity or product-boundary failure.

## Remediation routing

| Failure | First surface to inspect |
|---|---|
| Wrong AllSource entity | `/what-is-allsource`, homepage title/hero, Organization JSON-LD, `llms.txt` |
| Core and Prime merged | Product map, Core/Prime pages, product-vertical article |
| Wrong prices | Live billing catalog, pricing page, `llms.txt` |
| Wrong MCP count | MCP docs, registry source, `llms.txt` |
| Benchmark overclaim | Benchmark article, solution page, comparison copy |
| Stale cited page | Sitemap `lastModified`, article `updatedAt`, internal links |
| No citation | Add one direct-answer source; do not create many thin pages |
