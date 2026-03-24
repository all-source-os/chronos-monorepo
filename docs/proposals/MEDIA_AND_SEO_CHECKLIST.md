# Media Production & SEO Checklist — Marketing Readiness

> **Status**: Active
> **Date**: 2026-03-24
> **Goal**: Every page on all-source.xyz renders correctly, has proper SEO, and has real visual assets

---

## CRITICAL: Missing Assets Blocking Production

These are referenced in code but **do not exist**:

| Asset | Referenced By | Times | Impact |
|-------|-------------|-------|--------|
| `/dashboard.png` | All 13 blog posts, hero section, OG image generator, solution pages | 30+ | Every blog post has broken OG image, hero has broken visual |
| `/author.jpg` | Blog post template (hardcoded) | 1 | Broken author avatar on every blog post |

**Action**: Create these two files or update all references.

---

## Part 1: Videos to Produce

### 1.1 Terminal Recordings (asciinema) — 3 recordings

| Recording | Command | Duration | Embed Location |
|-----------|---------|----------|----------------|
| `prime-graph.cast` | `cargo run --features prime --example prime_graph` | ~15s | `/docs/prime/quickstart` |
| `prime-vectors.cast` | `cargo run --features prime-full --example prime_vectors` | ~20s | `/docs/prime/embedded` |
| `prime-recall.cast` | `cargo run --features prime-recall --example prime_recall` | ~25s | `/docs/prime/concepts` |

**Tool**: `asciinema rec <file>.cast -c "<command>"`
**Playback speed**: 1.5x
**Embed**: `<asciinema-player>` component or GIF conversion

### 1.2 Demo Video: 60 seconds — Claude Desktop + Prime

**Storyboard**:
1. (0:00-0:10) User opens Claude Desktop, Prime MCP is connected
2. (0:10-0:20) User: "What do you know about our project?" → Claude calls `prime_index` → compressed index shown
3. (0:20-0:30) User: "Alice is now leading the security team" → Claude calls `prime_add_node` + `prime_add_edge`
4. (0:30-0:45) User: "How does security relate to engineering?" → Claude calls `prime_context` → cross-domain result
5. (0:45-0:60) Tagline: "cargo install allsource-prime" + GitHub link

**Requirements**:
- Captions (SRT file) for muted autoplay on social
- 1920x1080 or 1080x1080 (square for X/Twitter)
- Dark theme Claude Desktop

**Distribution**: X thread #2, YouTube, blog embed, `/solutions/agent-memory`

### 1.3 Technical Walkthrough: 5 minutes

**Outline**:
1. (0:00-0:30) Problem: agents forget, 3-database problem
2. (0:30-1:30) Architecture: everything is an event (show diagram)
3. (1:30-2:30) Projections: show `Projection` trait, DashMap lookups
4. (2:30-3:30) Compressed index: `build_heuristic_index()`, auto-generation
5. (3:30-4:30) Hybrid recall: `RecallEngine.context()`, cross-domain
6. (4:30-5:00) Benchmarks + install command

**Requirements**: Large font, dark IDE theme, code readable at 720p

---

## Part 2: Architecture Diagrams to Create

### 2.1 "One Engine, Not Three Databases" — PRIMARY

**Currently**: ASCII art in `agent-memory/page.tsx` and blog posts
**Need**: Polished SVG/PNG, 1200x600px

```
┌─────────────────────────────────────────────┐
│              AllSource Prime                 │
│  Graph    Vectors    Temporal    Compressed   │
│  ┌──────────────────────────────────────┐   │
│  │  WAL + Parquet + DashMap + HLC + CRDT │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

**Use in**: Homepage hero, `/solutions/agent-memory`, blog posts, X thread #3

### 2.2 "Everything Is an Event" — Event Flow

**Shows**: `prime.node.created` → Projections → Query Results
**Use in**: `building-agent-memory-in-rust.mdx`, `/docs/prime/concepts`

### 2.3 "zer0dex vs AllSource" — Side-by-Side

**Shows**: Two architectures compared visually (not just table)
**Use in**: `zer0dex-vs-allsource-recall.mdx`, X thread #1

### 2.4 "Recall Pipeline" — Data Flow

**Shows**: Domain events → DomainIndex → CrossDomain → CompressedIndex → RecallEngine → Response
**Use in**: `compressed-index-doubles-cross-domain-recall.mdx`, `/docs/prime/concepts`

### 2.5 "Agent Memory Problem" — Before/After

**Shows**: Current state (ephemeral context window) vs Prime (persistent graph)
**Use in**: `ai-agents-need-memory.mdx`, homepage

---

## Part 3: Static Images to Create

### 3.1 Dashboard Hero Screenshot — `/public/dashboard.png`

**THE most critical missing asset.** Referenced 30+ times.

**Should show**: Real Prime dashboard or mockup with:
- Graph visualization (nodes colored by domain)
- Compressed index panel
- Query results with relevance scores
- Dark theme matching the site

**Size**: 1920x1080 (displayed at various sizes)

### 3.2 Blog Post OG Images — 13 unique images

Each blog post currently uses `/dashboard.png` (which doesn't exist). Need unique OG images:

| # | Blog Post | OG Image Concept |
|---|-----------|-----------------|
| 1 | Introducing AllSource | Logo + "Time-travel your data" tagline |
| 2 | AI Agents Need Memory | Brain/memory icon + context window visualization |
| 3 | zer0dex vs AllSource | Split comparison visual |
| 4 | Building Agent Memory in Rust | Rust logo + architecture diagram |
| 5 | Compressed Index Doubles Recall | Chart: 37.5% → 80% bar graph |
| 6 | 12μs Agent Memory | Speedometer/latency visualization |
| 7 | From zer0dex to AllSource | Arrow from one to the other |
| 8 | Time-Travel Queries | Timeline visualization |
| 9 | Why Event Sourcing 2026 | Event stream visualization |
| 10 | Temporal AI: Future of RAG | AI + temporal data mashup |
| 11 | Event Store vs Database | Two icons compared |
| 12 | MCP Tools + Claude | Claude logo + tools grid |
| 13 | Tiered Context Loading | L0/L1/L2 tier pyramid |

**Size**: 1200x630px (standard OG), PNG
**Branding**: AllSource logo + purple-blue gradient + white text

### 3.3 Author Avatar — `/public/author.jpg`

**Currently**: Hardcoded reference to `/author.jpg` in blog template
**Need**: AllSource team avatar or logo mark
**Size**: 100x100px minimum

### 3.4 Docs Hero Banners — 5 images

| Page | Banner Concept |
|------|---------------|
| `/docs/prime/quickstart` | Terminal with "cargo install" |
| `/docs/prime/concepts` | Projection architecture |
| `/docs/prime/mcp` | Claude Desktop integration |
| `/docs/prime/http` | API endpoint curl examples |
| `/docs/prime/embedded` | Rust code with crate import |

**Size**: 1200x400px, dark theme

---

## Part 4: SEO Fixes Required

### 4.1 CRITICAL: Missing robots.txt

**Action**: Create `/apps/web/public/robots.txt`

```
User-agent: *
Allow: /
Disallow: /dashboard/
Disallow: /api/
Disallow: /onboarding/

Sitemap: https://www.all-source.xyz/sitemap.xml
```

### 4.2 CRITICAL: Sitemap incomplete

**Current state**: Only includes `/` and `/blog/*` posts.
**Missing**:
- `/docs` and all `/docs/*` subpages
- `/solutions/agent-memory`
- `/solutions/quant-intelligence`
- `/privacy`, `/terms`, `/changelog`, `/status`

**Action**: Update `apps/web/src/app/sitemap.ts` to include all public pages.

### 4.3 Missing metadata on "use client" pages

These pages have NO server-side metadata (title, description, OG tags):

| Page | Fix |
|------|-----|
| `/solutions/agent-memory` | Convert to server component or add `generateMetadata` |
| `/solutions/quant-intelligence` | Same |
| `/status` | Same |
| `/` (homepage) | Add explicit metadata export |

**Impact**: Google shows generic title, no OG preview on social sharing.

### 4.4 Missing canonical URLs

No pages have explicit `<link rel="canonical">` tags.

**Action**: Add to `constructMetadata()` in `utils.ts`:
```ts
alternates: {
  canonical: `${siteConfig.url}${pathname}`,
}
```

### 4.5 Hardcoded stats inconsistencies

| Stat | Location 1 | Location 2 | Correct Value |
|------|-----------|-----------|---------------|
| MCP tools | "27" (hero, features, 20+ places) | "43" (config.ts) | **13 Prime + 61 Elixir = 74 total** |
| Version | "v0.10.0" (hero pill) | "0.17.0" (Cargo.toml) | **v0.17.0** |
| Performance | "469K events/sec" | Same everywhere | Current (verify with latest bench) |
| Latency | "11.9μs" → "12μs" | Mixed | Standardize to one |

### 4.6 Structured data (JSON-LD) gaps

**Has JSON-LD**: Blog posts (BlogPosting schema)
**Missing JSON-LD**:
- Homepage: `Organization` + `SoftwareApplication` schema
- Solutions pages: `Product` schema
- Docs pages: `TechArticle` schema

### 4.7 Image alt text audit

Many images use generic or missing alt text. The `dashboard.png` references have alt text like "AllSource Dashboard" which is fine, but images in blog posts and marketing pages should have descriptive alt text for accessibility and SEO.

---

## Part 5: Social Media Assets

### 5.1 X/Twitter Launch Campaign — 3 threads

| Thread | Visual Needs | Status |
|--------|-------------|--------|
| #1 Launch (10 tweets) | Architecture diagram (2.1) | Text drafted |
| #2 Demo (4 tweets) | 60-second video (1.2) + benchmark chart | Video TODO |
| #3 Technical (5 tweets) | Code screenshots + diagram (2.2) | TODO |

### 5.2 Benchmark Publication Visuals

| Chart | Data | Format |
|-------|------|--------|
| Cross-domain recall bar chart | zer0dex 80%, RAG 37.5%, Prime 80%+ | PNG 1200x600 |
| Latency comparison | zer0dex 70ms, Mem0 200ms+, Prime 12μs | PNG 1200x600 |
| Feature matrix card | 7 features × 5 systems | PNG 1200x800 |

---

## Production Priority Order

### Must-have before launch (blocks marketing):
1. [ ] **`/public/dashboard.png`** — real screenshot or high-quality mockup
2. [ ] **`/public/author.jpg`** — team avatar
3. [ ] **`robots.txt`** — create and deploy
4. [ ] **Fix sitemap.ts** — add all public pages
5. [ ] **Fix metadata** on solutions + homepage pages
6. [ ] **Fix version number** in hero pill (v0.10.0 → v0.17.0)
7. [ ] **Fix MCP tools count** — resolve 27 vs 43 discrepancy

### Should-have for launch week:
8. [ ] 3 terminal recordings (asciinema)
9. [ ] 60-second demo video
10. [ ] 5 architecture diagrams (SVG)
11. [ ] 13 unique blog OG images
12. [ ] Canonical URLs on all pages

### Nice-to-have post-launch:
13. [ ] 5-minute technical walkthrough
14. [ ] Interactive playground component
15. [ ] Docs hero banners
16. [ ] JSON-LD structured data on non-blog pages
17. [ ] Benchmark publication visuals
18. [ ] Code screenshots for blog posts
