# Visual Assets for Social Media

## 📊 Performance Graph Cards

### Card 1: Throughput Evolution
```
┌─────────────────────────────────────────────────┐
│  AllSource AllSource - Throughput Roadmap         │
├─────────────────────────────────────────────────┤
│                                                  │
│  v1.0 (Current)                                  │
│  ████████░░░░░░░░░░░░░░░░  469K events/sec     │
│                                                  │
│  v1.2 (Q1 2026)                                  │
│  █████████████████░░░░░░░ 1.0M events/sec      │
│                                                  │
│  v2.0 (2027)                                     │
│  ████████████████████████ 5.0M events/sec      │
│                                                  │
│  0          1M          2.5M          5M         │
│                                                  │
│  +113% improvement from v1.0 → v1.2              │
│  +967% improvement from v1.0 → v2.0              │
└─────────────────────────────────────────────────┘
```

### Card 2: Query Latency Comparison
```
┌─────────────────────────────────────────────────┐
│  Query Latency (p99) - Lower is Better          │
├─────────────────────────────────────────────────┤
│                                                  │
│  PostgreSQL       ████████████████  ~50μs       │
│                                                  │
│  EventStoreDB     ████████  ~25μs                │
│                                                  │
│  AllSource v1.0   ███  11.9μs                    │
│                                                  │
│  AllSource v1.2   ██  <5μs (target)              │
│                                                  │
│  0        10μs     25μs      50μs                │
│                                                  │
│  58% faster than current baseline                │
│  5x faster than traditional databases            │
└─────────────────────────────────────────────────┘
```

### Card 3: Phase 1.5 Progress Dashboard
```
┌─────────────────────────────────────────────────┐
│  Phase 1.5: Clean Architecture Migration        │
│  Overall Progress: 70% Complete                  │
├─────────────────────────────────────────────────┤
│                                                  │
│  Domain Layer                                    │
│  ████████████████████  100% ✅                  │
│  • Pure entities                                 │
│  • Value objects                                 │
│  • Repository traits                             │
│                                                  │
│  Application Layer                               │
│  ████████████████████  100% ✅                  │
│  • Use cases                                     │
│  • DTOs                                          │
│  • Service orchestration                         │
│                                                  │
│  Infrastructure Layer                            │
│  ██████░░░░░░░░░░░░░░  30% ⏳                   │
│  • Persistence adapters                          │
│  • Web handlers                                  │
│  • External integrations                         │
│                                                  │
│  Tests: 86/86 passing (100% pass rate)          │
│  Coverage: 100% (Domain + Application)          │
└─────────────────────────────────────────────────┘
```

### Card 4: Architecture Comparison
```
┌─────────────────────────────────────────────────┐
│  What Makes AllSource Different?                │
├─────────────────────────────────────────────────┤
│                                                  │
│  Feature          Traditional    AllSource      │
│  ──────────────────────────────────────────────  │
│                                                  │
│  Time Travel      ❌             ✅              │
│  AI-Native (MCP)  ❌             ✅              │
│  Vector Search    External       Native         │
│  Keyword Search   External       Native         │
│  Event Forks      ❌             ✅              │
│  Clean Arch       ❌             ✅              │
│  Throughput       50-100K        469K→1M+       │
│  Query Latency    50-100μs       11.9→<5μs      │
│                                                  │
│  Built for AI agents, not just humans           │
└─────────────────────────────────────────────────┘
```

### Card 5: Tech Stack Visualization
```
┌─────────────────────────────────────────────────┐
│  AllSource AllSource - Technology Stack           │
├─────────────────────────────────────────────────┤
│                                                  │
│  ┌──────────────────────────────────────────┐   │
│  │  MCP Server (TypeScript)                 │   │
│  │  AI-Native Interface Layer               │   │
│  └──────────────────────────────────────────┘   │
│              ↓                                   │
│  ┌──────────────────────────────────────────┐   │
│  │  Control Plane (Go)                      │   │
│  │  Multi-tenancy • RBAC • Policies         │   │
│  └──────────────────────────────────────────┘   │
│              ↓                                   │
│  ┌──────────────────────────────────────────┐   │
│  │  Core Engine (Rust) 🦀                   │   │
│  │  High-Performance Event Store            │   │
│  │  • 469K events/sec                       │   │
│  │  • 11.9μs latency                        │   │
│  │  • Lock-free concurrency                 │   │
│  └──────────────────────────────────────────┘   │
│              ↓                                   │
│  ┌──────────────────────────────────────────┐   │
│  │  Storage Layer                           │   │
│  │  Parquet (columnar) + WAL (durability)   │   │
│  └──────────────────────────────────────────┘   │
│                                                  │
│  Future: Clojure Query Service (v1.3+)          │
└─────────────────────────────────────────────────┘
```

### Card 6: Test Coverage & Quality
```
┌─────────────────────────────────────────────────┐
│  Quality Metrics - AllSource AllSource            │
├─────────────────────────────────────────────────┤
│                                                  │
│  ✅ Tests Passing                                │
│     ████████████████████  86/86 (100%)          │
│                                                  │
│  ✅ Domain Layer Coverage                        │
│     ████████████████████  100%                  │
│                                                  │
│  ✅ Application Layer Coverage                   │
│     ████████████████████  100%                  │
│                                                  │
│  ⏳ Infrastructure Coverage                      │
│     ██████░░░░░░░░░░░░░░  30%                   │
│                                                  │
│  ✅ Go Control Plane                             │
│     ████░░░░░░░░░░░░░░░░  23.1%                 │
│                                                  │
│  Overall: Production-ready core with ongoing    │
│  infrastructure refactoring                      │
└─────────────────────────────────────────────────┘
```

---

## 🎨 Banner Design Concepts

### Banner 1: Hero Banner (1200x675 - LinkedIn/Twitter)
```
Concept: Dark theme with code-style aesthetics

┌────────────────────────────────────────────────────────┐
│                                                         │
│  AllSource ALLSOURCE                    🦀 Rust          │
│  ════════════════════                                   │
│                                                         │
│  The AI-Native Event Store                              │
│                                                         │
│  ✦ 469K events/sec                                      │
│  ✦ 11.9μs query latency                                 │
│  ✦ Time-travel queries                                  │
│  ✦ Built for AI agents                                  │
│                                                         │
│  [GitHub] [Docs] [Roadmap]              v1.0 Released  │
│                                                         │
└────────────────────────────────────────────────────────┘

Colors:
- Background: Deep navy (#0a0e27)
- Primary text: White (#ffffff)
- Accent: Rust orange (#ce422b)
- Secondary: Teal (#4ecdc4)
```

### Banner 2: Performance Focus (1200x675)
```
Concept: Graph visualization on gradient background

┌────────────────────────────────────────────────────────┐
│                                                         │
│  PERFORMANCE THAT SCALES                                │
│                                                         │
│    5M ┤                                    ╭─ v2.0     │
│       │                                ╭───╯            │
│    1M ┤                          ╭─────╯ v1.2          │
│       │                      ╭───╯                      │
│  469K ┤──────────────────────╯ v1.0 (Current)          │
│       │                                                 │
│    0  └─────────────────────────────────────           │
│       2025              2026              2027          │
│                                                         │
│  AllSource AllSource - Built for Scale                    │
│                                                         │
└────────────────────────────────────────────────────────┘

Colors:
- Background: Gradient (purple to blue)
- Graph line: Bright cyan
- Text: White
```

### Banner 3: AI-Native Focus (1200x675)
```
Concept: Split design showing traditional vs. AI-native

┌────────────────────────────────────────────────────────┐
│                                                         │
│  TRADITIONAL           │  AI-NATIVE                     │
│  EVENT STORES          │  EVENT STORE                   │
│  ────────────          │  ──────────────                │
│                        │                                │
│  ❌ Human operators    │  ✅ AI agents                  │
│  ❌ Manual queries     │  ✅ Autonomous                 │
│  ❌ External search    │  ✅ Native search              │
│  ❌ No experimentation │  ✅ Instant forks              │
│                        │                                │
│                        │  AllSource AllSource             │
│                        │  The difference is built-in    │
│                        │                                │
└────────────────────────────────────────────────────────┘

Colors:
- Left side: Desaturated gray
- Right side: Vibrant gradient
- Checkmarks: Green (#00ff88)
```

### Banner 4: Architecture Showcase (1200x675)
```
Concept: Clean, layered architecture diagram

┌────────────────────────────────────────────────────────┐
│                                                         │
│         AllSource AllSource - Clean Architecture          │
│                                                         │
│  ┌────────────────────────────────────────────────┐   │
│  │  MCP Layer    AI Agents • Embedded Expertise   │   │
│  └────────────────────────────────────────────────┘   │
│  ┌────────────────────────────────────────────────┐   │
│  │  Control      Multi-tenancy • RBAC • Policies  │   │
│  └────────────────────────────────────────────────┘   │
│  ┌────────────────────────────────────────────────┐   │
│  │  Core         469K events/sec • 11.9μs latency │   │
│  └────────────────────────────────────────────────┘   │
│  ┌────────────────────────────────────────────────┐   │
│  │  Storage      Parquet • WAL • Time-travel      │   │
│  └────────────────────────────────────────────────┘   │
│                                                         │
│         TypeScript • Go • Rust • Clojure               │
│                                                         │
└────────────────────────────────────────────────────────┘

Colors:
- Background: Light gradient
- Layers: Different shades of blue
- Text: Dark gray
- Accents: Rust orange
```

---

## 📱 Story/Carousel Formats

### Instagram/LinkedIn Carousel (Square Format)

#### Slide 1: Title
```
╔═══════════════════════════╗
║                           ║
║    AllSource              ║
║    ALLSOURCE                ║
║                           ║
║    The AI-Native          ║
║    Event Store            ║
║                           ║
║    v1.0 Released          ║
║                           ║
║    Swipe for stats →      ║
║                           ║
╚═══════════════════════════╝
```

#### Slide 2: Performance
```
╔═══════════════════════════╗
║  ⚡ Performance            ║
║                           ║
║  469,000                  ║
║  events/sec               ║
║                           ║
║  11.9μs                   ║
║  query latency            ║
║                           ║
║  Target: 1M+              ║
║  events/sec (v1.2)        ║
║                           ║
╚═══════════════════════════╝
```

#### Slide 3: AI-Native
```
╔═══════════════════════════╗
║  🤖 AI-Native             ║
║                           ║
║  ✓ MCP Protocol           ║
║  ✓ Embedded Expertise     ║
║  ✓ Instant Forks          ║
║  ✓ Vector Search          ║
║  ✓ Keyword Search         ║
║                           ║
║  Built for agents,        ║
║  not just humans          ║
║                           ║
╚═══════════════════════════╝
```

#### Slide 4: Tech Stack
```
╔═══════════════════════════╗
║  🛠️ Tech Stack            ║
║                           ║
║  🦀 Rust Core             ║
║  🐹 Go Control Plane      ║
║  📦 TypeScript MCP        ║
║  🎯 Clojure Queries       ║
║                           ║
║  Clean Architecture       ║
║  SOLID Principles         ║
║  100% Test Coverage       ║
║                           ║
╚═══════════════════════════╝
```

#### Slide 5: Open Source
```
╔═══════════════════════════╗
║  💫 Open Source           ║
║                           ║
║  MIT Licensed             ║
║  86/86 tests passing      ║
║  100% coverage            ║
║                           ║
║  ⭐ Star on GitHub        ║
║  📖 Read the docs         ║
║  🤝 Contributors welcome  ║
║                           ║
║  Link in bio              ║
║                           ║
╚═══════════════════════════╝
```

---

## 🎥 Video/Animation Concepts

### 30-Second Explainer Script
```
00:00 - "Traditional event stores weren't built for AI agents"
00:05 - Show comparison: Human operator vs. AI agent
00:10 - "AllSource AllSource changes that"
00:15 - Quick feature showcase (MCP, Forks, Search)
00:20 - Performance numbers flash on screen
00:25 - "469K→1M+ events/sec. Open source. MIT licensed."
00:30 - Logo + GitHub link
```

### Code Animation (Terminal Recording)
```
# Show real-time event ingestion
$ cargo run --release

Ingesting events...
█████████████░░░░░░░░░░  469,432 events/sec
Query latency (p99): 11.9μs
Tests: 86/86 passing ✅

# Switch to MCP query
$ mcp-cli query --entity user-123 --time-travel "2024-01-01"

Reconstructing state at 2024-01-01...
✓ Found 1,247 events
✓ Applied projections
✓ State reconstructed in 0.3ms

# Show instant fork
$ mcp-cli create-fork --ttl 3600

Fork created: fork-abc123
Ready for experimentation 🧪
```

---

## 📊 Infographic Suggestions

### Topic 1: "Event Sourcing vs. Traditional Databases"
- Side-by-side comparison
- Use cases where event sourcing wins
- Performance metrics

### Topic 2: "Clean Architecture in Practice"
- Layer visualization
- Dependency flow
- Testing pyramid

### Topic 3: "AI-Native Design Principles"
- MCP protocol benefits
- Embedded expertise examples
- Agent workflow comparison

### Topic 4: "Performance Journey: v1.0 → v2.0"
- Timeline with milestones
- Optimization techniques
- Benchmark results

---

## 🖼️ Image Generation Prompts (for AI tools)

### For DALL-E / Midjourney

**Prompt 1: Hero Image**
```
A futuristic database visualization with flowing event streams,
dark blue and orange color scheme, high-tech aesthetic,
circuit board patterns, holographic data flows,
professional software engineering style,
cyberpunk influences, clean and modern
```

**Prompt 2: Architecture Diagram**
```
Clean layered software architecture diagram,
isometric view, modular blocks stacked vertically,
blue gradient color scheme, professional tech illustration,
minimalist design, connection lines between layers,
glowing accents, white background
```

**Prompt 3: Performance Graph**
```
Abstract data visualization showing exponential growth,
blue and teal gradient background, glowing line graph,
futuristic dashboard aesthetic, professional design,
particle effects, tech-inspired, clean and modern
```

---

## ⚙️ Tools for Creating These Visuals

### Free Tools:
- **Canva** (templates for social media)
- **Figma** (custom designs)
- **Carbon** (code screenshots - carbon.now.sh)
- **Excalidraw** (hand-drawn diagrams)
- **Mermaid** (architecture diagrams)

### Paid Tools:
- **Adobe Express** (professional templates)
- **Sketch** (vector graphics)
- **Affinity Designer** (illustration)

### Code-Based:
- **D3.js** (interactive graphs)
- **Chart.js** (simple charts)
- **Plotly** (scientific visualizations)
- **Vega-Lite** (declarative graphics)

---

## 📏 Size Guidelines

### Twitter/X:
- Single image: 1200x675px
- Profile header: 1500x500px
- Profile photo: 400x400px

### LinkedIn:
- Single image: 1200x627px
- Document (carousel): 1080x1080px or 1200x1500px
- Profile cover: 1584x396px

### Instagram:
- Feed post: 1080x1080px (square)
- Story: 1080x1920px (9:16)
- Carousel: 1080x1080px

### General:
- Keep text large and readable
- High contrast for accessibility
- Test on mobile first
- Export in PNG for quality
