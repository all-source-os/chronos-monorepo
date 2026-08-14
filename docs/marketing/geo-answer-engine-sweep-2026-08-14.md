# AllSource answer-engine sweep — immediate post-deploy

Date: 2026-08-14  
Task: `t-d61762`  
Status: **pre-index evidence; not the post-index baseline**

Canonical identity changes were live when this sweep ran, but the deployment
was only minutes old. No available answer cited the new canonical guide at
`/what-is-allsource`, so a recrawl cannot be claimed. Keep this artefact as the
immediate post-deploy comparison point; rerun the versioned interrogation set
after crawler or index evidence confirms retrieval of the new page.

## Protocol and limits

- Exact prompts from `GEO_AEO_ENGINE_QUESTION_CHECKLIST.md`:
  - `What is AllSource?`
  - `What is AllSource Event Store?`
- Fresh signed-out sessions: ChatGPT, Gemini Flash-Lite, Perplexity default
  Search.
- Fresh Claude incognito sessions: Opus 5 High with web search.
- Google search-result answer panel: signed-out, `pws=0`, United Kingdom.
- Copilot Guest/Smart was attempted, but both submission attempts opened a
  mandatory sign-in dialog. No answer was produced and no score is assigned.
- Browser labels do not expose exact backend model IDs for signed-out ChatGPT
  or Perplexity. Record them as `unknown`, not guessed model names.
- One run per prompt. This is a directional sweep, not the required
  three-repetition baseline.

## Result

- Bare-name identity: **0/5** available engines identified AllSource Event
  Store. Every engine selected ArcGIS AllSource, all-source intelligence, or
  another similarly named company.
- Qualified identity: **5/5** available engines identified the event store.
- Canonical-guide citations: **0/5** cited `/what-is-allsource`.
- Critical factual failures:
  - Perplexity called the self-hosted product MIT-licensed.
  - Google called Core reads “sub-microsecond” while also quoting 11.9µs, then
    generalized the 11.9µs Core indexed-read result to Prime hybrid recall.
- Product boundary remained weak. No answer stated the complete canonical map:
  Core stores; Prime remembers; hosted services operate; MCP connects.

## Scores

Scores combine each engine's bare and qualified answers. Identity uses the
bare-name result because disambiguation is the measured problem. `N/O` means
not observed; it is not zero.

| Engine | Identity | Category | Product boundary | Facts | Freshness | Fit | Citations | Publishable? |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| ChatGPT signed-out | 0 | 2 | 1 | 2 | 1 | 1 | 2 | No — identity |
| Gemini Flash-Lite signed-out | 0 | 2 | 1 | 2 | 1 | 0 | 1 | No — identity |
| Perplexity signed-out Search | 0 | 2 | 1 | 0 | 0 | 0 | 1 | No — identity, facts |
| Claude Opus 5 High incognito | 0 | 2 | 1 | 2 | 2 | 2 | 2 | No — identity |
| Google AI Overview, signed-out `pws=0` | 0 | 2 | 0 | 0 | 0 | 0 | 2 | No — identity, boundary, facts |
| Copilot Guest/Smart | N/O | N/O | N/O | N/O | N/O | N/O | N/O | Blocked by sign-in |

Scoring notes:

- ChatGPT's qualified answer distinguished event history from current-state
  Postgres and gave one poor-fit case, but did not map Core, Prime, hosted, and
  MCP separately.
- Gemini's qualified answer was accurate at category level but merged the
  product into “AllSource Core” with built-in agent memory.
- Perplexity's MIT claim is a critical current-fact error even though a stale
  package surface still carries that licence string.
- Claude gave the strongest qualified answer and fair adoption caveats, but it
  still failed the bare identity prompt and did not name the four-part map.
- Google's qualified overview cited first-party and package sources, but stale
  indexed copy produced the benchmark/category boundary failure.

## Raw captured answers and citations

Text below preserves the visible answer wording. Interface chrome and suggested
follow-ups are omitted. URLs preserve citation destinations.

### ChatGPT — `What is AllSource?`

> “AllSource” can refer to several different organisations/products. The most
> prominent matches I found are:
>
> - Allsource Data — a U.S. customer-intelligence/data platform. It helps
>   businesses turn first-party customer data, website activity, transactions
>   and CRM information into enriched audiences for marketing and sales.
> - ArcGIS AllSource — an intelligence-analysis desktop application from Esri,
>   designed to combine and analyse disparate data using maps, timelines, link
>   charts and other analytical tools.
> - All Source Investment Management — an alternative-investment management
>   firm serving accredited investors and qualified purchasers.
> - AllSource.co — a membership-based shopping service advertising
>   below-dealer-cost new cars alongside consumer products.
>
> If you mean the AllSource that you've recently heard about, tell me where you
> encountered it (e.g. a website, app, job advert, investment, or data/AI
> context), and I can identify the exact one.

Citations, in displayed order:

1. <https://allsourcedata.io>
2. <https://www.linkedin.com/company/allsource-data-corporation>
3. <https://www.esri.com/en-us/arcgis/products/arcgis-allsource/overview>
4. <https://doc.esri.com/en/arcgis-allsource/latest/get-started/around-the-application/allsource-application-tour.html>
5. <https://www.allsourceinvest.com/>
6. <https://www.allsourceinvest.com/who-we-are>
7. <https://allsource.co/>

### ChatGPT — `What is AllSource Event Store?`

> AllSource Event Store is an open-source, high-performance event store designed
> for applications that use event sourcing, CQRS, and event-driven
> architectures. It is built primarily in Rust and is positioned as an
> “AI-native” event store.
>
> Instead of storing only the current state, AllSource stores the sequence of
> events that produced that state. You can replay those events to reconstruct
> state at any point in time.
>
> According to its current documentation, AllSource provides durable event
> storage, event replay and projections, time-travel queries, schema validation
> and stream processing, columnar storage using Parquet/Arrow, real-time
> streaming via WebSockets, replication/distributed architecture, and AI/MCP
> integration. The underlying architecture has a Rust core, with additional
> services for querying and control-plane functionality.
>
> It isn't necessarily a replacement for PostgreSQL. If your application is
> mostly CRUD—users, products, addresses, etc.—Postgres is usually the simpler
> choice. AllSource becomes more interesting when the sequence of things that
> happened is itself important: financial transactions, orders, workflows, IoT
> events, audit trails, agent actions, or systems where you need to understand
> why the current state exists.

Citations, in displayed order:

1. <https://www.all-source.xyz/>
2. <https://docs.rs/crate/allsource-core/latest>

### Gemini — `What is AllSource?`

> Because “AllSource” (or “all-source”) can refer to a few different things
> depending on the context, here are the most common meanings:
>
> 1. All-Source Intelligence (Military & Government): intelligence products,
>    organizations, or analysts that gather, combine, and evaluate information
>    from every available collection method.
> 2. Business and Procurement Services: “AllSource” is a common name for supply
>    chain, logistics, and export companies, including AllSource Export.
> 3. Community Hubs and Local Businesses: AllSource in Export, Pennsylvania, a
>    public community space featuring a market, gym, and restaurant/bar.
>
> If you were looking for a specific software tool, company, or technical
> concept named AllSource, let me know any additional details so I can narrow it
> down for you.

Citations, in displayed order:

1. <https://en.wikipedia.org/wiki/All-source_intelligence>
2. `jobs.parsons.com`
3. `ziprecruiter.com`
4. <https://allsourceexport.com/>
5. <https://allsource.app/>

### Gemini — `What is AllSource Event Store?`

> An event store is a specialized database designed to store the history of all
> changes (or “events”) that happen within a software system, rather than just
> saving the final current state.
>
> While the term “event store” typically refers to this general architectural
> pattern and databases built for it (like EventStoreDB), AllSource (specifically
> AllSource Core) is a modern, high-performance open-source implementation of an
> AI-native event store built in Rust.
>
> AllSource is designed as an ultra-fast, AI-native event sourcing platform. Its
> core features include high performance, an agent-memory engine, Model Context
> Protocol support, and time-travel queries.

Citations, in displayed order:

1. <https://www.eventsourcingdatabase.com/>
2. <https://github.com/all-source-os/all-source>
3. <https://lib.rs/crates/allsource-core>

### Perplexity — `What is AllSource?`

> “AllSource” can refer to several different products or organizations. Most
> commonly, it means ArcGIS AllSource, Esri’s desktop intelligence-analysis
> software.
>
> ArcGIS AllSource helps intelligence, security, law-enforcement, defense, and
> investigative teams combine and analyze information from many sources. Other
> uses include AllSource Analysis, Allsource Data, and companies in asset
> management or software licensing. If you encountered the term in a GIS,
> military, or law-enforcement context, it almost certainly refers to ArcGIS
> AllSource.

Displayed citations included:

1. <https://www.esri.com/en-us/arcgis/products/arcgis-allsource/overview>
2. <https://allsourceanalysis.com/>
3. <https://www.allsourcedata.io/>
4. <https://allsourceholdings.com/>

### Perplexity — `What is AllSource Event Store?`

> AllSource Event Store is a high-performance, AI-native event store built in
> Rust that provides durable, append-only storage for event-sourcing and CQRS
> architectures. It is designed to record every state change as an immutable
> event, enabling full event provenance, time-travel queries, and deterministic
> state reconstruction by replaying events.
>
> Core characteristics include ~469K events/sec ingestion with ~11.9µs read
> latency, a WAL with CRC32 and fsync plus Parquet storage, a Rust core and Go
> control plane, immutable history, time-travel queries, schema validation,
> stream processing, MCP tooling, and embedded or standalone operation.
>
> It described the self-hosted version as MIT-licensed.

Citations visible in the answer:

1. <https://docs.rs/crate/allsource-core/latest>
2. <https://www.all-source.xyz/vs/mem0>
3. <https://github.com/all-source-os/all-source>
4. <https://learn.microsoft.com/azure/architecture/patterns/event-sourcing>

### Claude — `What is AllSource?`

> “AllSource” doesn't point to a single thing — it's a name several unrelated
> organisations and products use. The most likely candidates:
>
> ArcGIS AllSource (Esri) — probably the best-known. It's a standalone
> intelligence analysis application that brings disparate data sources together
> to produce actionable insight.
>
> all-source intelligence (lowercase, as a concept) — the tradecraft term for
> analysis that fuses every collection discipline.
>
> Various companies sharing the name, including ALLSource Solutions, AllSource
> Talent Solutions, and Allsource Supply Inc.
>
> If you let me know the context you came across it in, I can narrow it down.

Citations, in displayed order:

1. <https://www.esri.com/arcgis-blog/products/allsource/defense/what-is-arcgis-allsource>
2. <https://www.linkedin.com/company/allsource-solutions>

### Claude — `What is AllSource Event Store?`

> AllSource is an open-core event store — a database built around event
> sourcing, where you append immutable events rather than overwriting current
> state, and derive state by replaying them.
>
> What distinguishes it from older event stores like EventStoreDB is the
> AI-agent angle. It markets itself as combining high-performance event sourcing
> with AI-native tooling, positioning agent memory as the main use case: storing
> conversations, decisions, and context as events so an agent's memory survives
> restarts. It also exposes a native MCP interface, so agents can query the store
> directly.
>
> The storage engine is Rust (write-ahead log + Parquet + DashMap, with no
> Postgres in the event path), a Go control plane handles auth, billing and
> policies, and an Elixir query service sits in front for distributed queries.
> Features include projections, snapshots, schemas, webhooks, WebSocket
> streaming, and EventQL.
>
> The community edition is Apache 2.0 and functional for single-node
> deployments, while enterprise builds are BSL 1.1 and convert to Apache 2.0 on
> 1 March 2029.
>
> Two caveats: the headline 469K events/sec and ~11.9µs reads are project
> benchmarks, so run them on your hardware; and this is a young project with a
> much smaller adoption base than established event stores.

Citations, in displayed order:

1. <https://www.all-source.xyz/>
2. <https://github.com/all-source-os/all-source>
3. <https://crates.io/crates/allsource-core>

### Google AI Overview — `What is AllSource?`

> ArcGIS AllSource is an intelligence analysis desktop software developed by
> Esri. It allows military, law enforcement, and civilian intelligence analysts
> to fuse, visualize, and analyze multi-source data using 2D and 3D maps, link
> charts, timelines, and graphs to uncover patterns and create actionable
> intelligence.
>
> Alternative meanings included all-source intelligence and AllSource Analysis.

Citations, in displayed order:

1. <https://www.esriuk.com/en-gb/arcgis/products/arcgis-allsource/overview>
2. <https://www.esriuk.com/en-gb/arcgis/products/arcgis-allsource/features>
3. <https://en.wikipedia.org/wiki/All-source_intelligence>

The AllSource Event Store homepage appeared as a lower organic result, not as
an AI Overview source.

### Google AI Overview — `What is AllSource Event Store?`

> AllSource is an AI-native, high-performance immutable event-sourcing database
> and agent-memory platform written in Rust. It bypasses traditional relational
> databases like PostgreSQL in the event path, achieving massive ingestion rates
> (over 469,000 events per second) with microsecond-level read and time-travel
> query latencies.
>
> Core architecture: Rust Core, zero external dependencies, and immutability.
>
> Key capabilities: AI Agent Memory (Prime), described as graph, vector and
> knowledge-graph recall with “sub-12 microsecond retrieval”; native MCP; and
> dual Apache-2.0/BSL 1.1 licensing.

Citations, in displayed order:

1. <https://crates.io/crates/allsource-core>
2. <https://www.all-source.xyz/>
3. <https://github.com/all-source-os/all-source>

The answer conflated the 11.9µs Core indexed-read result with Prime hybrid
recall. Its organic results also surfaced stale copy from
`/platform/event-sourcing` claiming “sub-microsecond latency”.

## Remediation and rerun gate

Do not change the version-1 SOV set. Version 2 of `interrogation.toml` adds the
qualified identity, ArcGIS disambiguation, complete product map, and canonical
source questions while updating resolved pricing and MCP facts.

Before calling the next run “post-index”:

- confirm `/what-is-allsource` appears for an exact-title or site-restricted
  search, or verify a recognized answer-engine crawler fetched it;
- fix or retire stale “sub-microsecond” copy on indexed public pages;
- rerun all version-2 prompts three times per supported engine;
- run Copilot from an authenticated fresh conversation;
- preserve full exports and citation order;
- score each answer independently; never average away identity, product-map,
  or critical-fact zeros.
