# AllSource Visual Assets for Sales

## Brand Colors

```
Primary:    #ce422b (Rust/Fire - represents Rust core)
Secondary:  #00ADD8 (Go Cyan - represents Control Plane)
Tertiary:   #4E2A8E (Elixir Purple - represents Query/MCP)
Accent:     #4ecdc4 (Teal - represents AI/Innovation)
Dark:       #0a0e27 (Deep Navy - backgrounds)
Light:      #f8fafc (Off-white - text on dark)
```

---

## 1. Performance Metrics Visualization

### Throughput Comparison Chart (SVG)

```svg
<svg viewBox="0 0 600 400" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="allsourceGrad" x1="0%" y1="0%" x2="0%" y2="100%">
      <stop offset="0%" style="stop-color:#ce422b"/>
      <stop offset="100%" style="stop-color:#8b1a1a"/>
    </linearGradient>
  </defs>

  <!-- Background -->
  <rect width="600" height="400" fill="#0a0e27"/>

  <!-- Title -->
  <text x="300" y="35" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="20" font-weight="bold">
    Event Throughput Comparison (events/sec)
  </text>

  <!-- Y-axis labels -->
  <text x="45" y="80" text-anchor="end" fill="#888" font-family="system-ui" font-size="12">500K</text>
  <text x="45" y="140" text-anchor="end" fill="#888" font-family="system-ui" font-size="12">400K</text>
  <text x="45" y="200" text-anchor="end" fill="#888" font-family="system-ui" font-size="12">300K</text>
  <text x="45" y="260" text-anchor="end" fill="#888" font-family="system-ui" font-size="12">200K</text>
  <text x="45" y="320" text-anchor="end" fill="#888" font-family="system-ui" font-size="12">100K</text>

  <!-- Grid lines -->
  <line x1="55" y1="75" x2="580" y2="75" stroke="#333" stroke-width="1"/>
  <line x1="55" y1="135" x2="580" y2="135" stroke="#333" stroke-width="1"/>
  <line x1="55" y1="195" x2="580" y2="195" stroke="#333" stroke-width="1"/>
  <line x1="55" y1="255" x2="580" y2="255" stroke="#333" stroke-width="1"/>
  <line x1="55" y1="315" x2="580" y2="315" stroke="#333" stroke-width="1"/>
  <line x1="55" y1="350" x2="580" y2="350" stroke="#666" stroke-width="2"/>

  <!-- Bars -->
  <!-- AllSource: 469K -->
  <rect x="80" y="77" width="80" height="273" fill="url(#allsourceGrad)" rx="4"/>
  <text x="120" y="68" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="14" font-weight="bold">469K</text>

  <!-- EventStoreDB: ~100K -->
  <rect x="190" y="290" width="80" height="60" fill="#666" rx="4"/>
  <text x="230" y="282" text-anchor="middle" fill="#888" font-family="system-ui" font-size="12">~100K</text>

  <!-- Kafka: ~200K -->
  <rect x="300" y="230" width="80" height="120" fill="#666" rx="4"/>
  <text x="340" y="222" text-anchor="middle" fill="#888" font-family="system-ui" font-size="12">~200K</text>

  <!-- Marten: ~50K -->
  <rect x="410" y="320" width="80" height="30" fill="#666" rx="4"/>
  <text x="450" y="312" text-anchor="middle" fill="#888" font-family="system-ui" font-size="12">~50K</text>

  <!-- Custom: varies -->
  <rect x="520" y="290" width="50" height="60" fill="#444" rx="4"/>
  <text x="545" y="282" text-anchor="middle" fill="#666" font-family="system-ui" font-size="10">varies</text>

  <!-- X-axis labels -->
  <text x="120" y="375" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="12" font-weight="bold">AllSource</text>
  <text x="230" y="375" text-anchor="middle" fill="#888" font-family="system-ui" font-size="11">EventStoreDB</text>
  <text x="340" y="375" text-anchor="middle" fill="#888" font-family="system-ui" font-size="11">Kafka</text>
  <text x="450" y="375" text-anchor="middle" fill="#888" font-family="system-ui" font-size="11">Marten</text>
  <text x="545" y="375" text-anchor="middle" fill="#666" font-family="system-ui" font-size="10">Custom</text>
</svg>
```

### Query Latency Chart (SVG)

```svg
<svg viewBox="0 0 600 350" xmlns="http://www.w3.org/2000/svg">
  <rect width="600" height="350" fill="#0a0e27"/>

  <text x="300" y="35" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="20" font-weight="bold">
    Query Latency p99 (microseconds)
  </text>

  <!-- Latency bars (lower is better) -->
  <!-- AllSource: 11.9us -->
  <rect x="80" y="80" width="100" height="40" fill="#4ecdc4" rx="4"/>
  <text x="200" y="105" fill="#f8fafc" font-family="system-ui" font-size="14" font-weight="bold">11.9 us - AllSource</text>

  <!-- Others typically 100us-10ms -->
  <rect x="80" y="140" width="350" height="30" fill="#666" rx="4"/>
  <text x="450" y="160" fill="#888" font-family="system-ui" font-size="12">~100us - EventStoreDB</text>

  <rect x="80" y="185" width="400" height="30" fill="#555" rx="4"/>
  <text x="500" y="205" fill="#888" font-family="system-ui" font-size="12">~500us - Kafka</text>

  <rect x="80" y="230" width="500" height="30" fill="#444" rx="4"/>
  <text x="300" y="290" fill="#666" font-family="system-ui" font-size="12">1-10ms typical for SQL-based</text>

  <!-- Legend -->
  <text x="300" y="330" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="16">
    AllSource: 8-40x faster than alternatives
  </text>
</svg>
```

---

## 2. Architecture Overview (Simplified Sales Version)

```svg
<svg viewBox="0 0 800 500" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <filter id="shadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="2" dy="2" stdDeviation="3" flood-opacity="0.3"/>
    </filter>
  </defs>

  <rect width="800" height="500" fill="#0a0e27"/>

  <!-- Title -->
  <text x="400" y="40" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="24" font-weight="bold">
    AllSource Architecture
  </text>
  <text x="400" y="65" text-anchor="middle" fill="#888" font-family="system-ui" font-size="14">
    Polyglot Design: Best Language for Each Job
  </text>

  <!-- Users/Clients Layer -->
  <rect x="50" y="90" width="700" height="60" fill="#1a1f36" rx="8" filter="url(#shadow)"/>
  <text x="400" y="125" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="14">
    Web Dashboard | REST API | WebSocket | AI Agents (MCP)
  </text>

  <!-- Arrow down -->
  <path d="M400 150 L400 170 L390 160 M400 170 L410 160" stroke="#4ecdc4" stroke-width="2" fill="none"/>

  <!-- Control Plane (Go) -->
  <rect x="200" y="180" width="400" height="70" fill="#00ADD8" rx="8" filter="url(#shadow)"/>
  <text x="400" y="210" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">
    Control Plane (Go)
  </text>
  <text x="400" y="235" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="12">
    Auth | RBAC | Audit | Routing | OpenTelemetry
  </text>

  <!-- Arrows down -->
  <path d="M300 250 L300 280" stroke="#4ecdc4" stroke-width="2"/>
  <path d="M500 250 L500 280" stroke="#4ecdc4" stroke-width="2"/>

  <!-- Core Services Row -->
  <!-- Rust Core -->
  <rect x="50" y="290" width="220" height="100" fill="#ce422b" rx="8" filter="url(#shadow)"/>
  <text x="160" y="325" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">
    Event Store (Rust)
  </text>
  <text x="160" y="350" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">469K events/sec</text>
  <text x="160" y="370" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">11.9us latency</text>

  <!-- Query Service -->
  <rect x="290" y="290" width="220" height="100" fill="#4E2A8E" rx="8" filter="url(#shadow)"/>
  <text x="400" y="325" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">
    Query Service (Elixir)
  </text>
  <text x="400" y="350" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Real-time Projections</text>
  <text x="400" y="370" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">WebSocket Streaming</text>

  <!-- MCP Server -->
  <rect x="530" y="290" width="220" height="100" fill="#4E2A8E" rx="8" filter="url(#shadow)"/>
  <text x="640" y="325" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">
    MCP Server (Elixir)
  </text>
  <text x="640" y="350" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">27 AI-Native Tools</text>
  <text x="640" y="370" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Natural Language Queries</text>

  <!-- Storage Layer -->
  <rect x="50" y="420" width="700" height="60" fill="#1a1f36" rx="8" filter="url(#shadow)"/>
  <text x="400" y="455" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="14">
    Parquet Storage | Write-Ahead Log | PostgreSQL | Redis
  </text>

  <!-- Arrows to storage -->
  <path d="M160 390 L160 420" stroke="#888" stroke-width="1"/>
  <path d="M400 390 L400 420" stroke="#888" stroke-width="1"/>
</svg>
```

---

## 3. Feature Highlight Cards

### AI-Native Integration Card

```svg
<svg viewBox="0 0 400 250" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="250" fill="#0a0e27" rx="12"/>
  <rect x="10" y="10" width="380" height="230" fill="#1a1f36" rx="8"/>

  <!-- Icon placeholder -->
  <circle cx="50" cy="50" r="25" fill="#4ecdc4"/>
  <text x="50" y="55" text-anchor="middle" fill="#0a0e27" font-family="system-ui" font-size="20">AI</text>

  <!-- Title -->
  <text x="90" y="45" fill="#f8fafc" font-family="system-ui" font-size="18" font-weight="bold">AI-Native First</text>
  <text x="90" y="65" fill="#4ecdc4" font-family="system-ui" font-size="12">Model Context Protocol</text>

  <!-- Features list -->
  <text x="30" y="100" fill="#f8fafc" font-family="system-ui" font-size="13">27 tools for Claude/GPT agents</text>
  <text x="30" y="125" fill="#f8fafc" font-family="system-ui" font-size="13">Natural language event queries</text>
  <text x="30" y="150" fill="#f8fafc" font-family="system-ui" font-size="13">Pattern detection & analysis</text>
  <text x="30" y="175" fill="#f8fafc" font-family="system-ui" font-size="13">TOON format (50% fewer tokens)</text>

  <!-- Stat -->
  <text x="200" y="220" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="24" font-weight="bold">
    Only event store with native MCP
  </text>
</svg>
```

### Performance Card

```svg
<svg viewBox="0 0 400 250" xmlns="http://www.w3.org/2000/svg">
  <rect width="400" height="250" fill="#0a0e27" rx="12"/>
  <rect x="10" y="10" width="380" height="230" fill="#1a1f36" rx="8"/>

  <!-- Stats -->
  <text x="200" y="60" text-anchor="middle" fill="#ce422b" font-family="system-ui" font-size="48" font-weight="bold">469K</text>
  <text x="200" y="85" text-anchor="middle" fill="#888" font-family="system-ui" font-size="14">events per second</text>

  <line x1="50" y1="110" x2="350" y2="110" stroke="#333" stroke-width="1"/>

  <text x="120" y="150" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="32" font-weight="bold">11.9us</text>
  <text x="120" y="175" text-anchor="middle" fill="#888" font-family="system-ui" font-size="12">p99 latency</text>

  <text x="280" y="150" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="32" font-weight="bold">129MB</text>
  <text x="280" y="175" text-anchor="middle" fill="#888" font-family="system-ui" font-size="12">total footprint</text>

  <text x="200" y="220" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="14">
    Rust-powered. Lock-free. SIMD-accelerated.
  </text>
</svg>
```

---

## 4. Tech Stack Visualization

```svg
<svg viewBox="0 0 700 300" xmlns="http://www.w3.org/2000/svg">
  <rect width="700" height="300" fill="#0a0e27"/>

  <text x="350" y="35" text-anchor="middle" fill="#f8fafc" font-family="system-ui" font-size="20" font-weight="bold">
    Polyglot Architecture: Best Tool for Each Job
  </text>

  <!-- Rust -->
  <rect x="30" y="70" width="150" height="180" fill="#ce422b" rx="8"/>
  <text x="105" y="100" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">Rust</text>
  <text x="105" y="125" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Event Store Core</text>
  <line x1="50" y1="140" x2="160" y2="140" stroke="#fff" stroke-opacity="0.3"/>
  <text x="105" y="160" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Zero-cost abstractions</text>
  <text x="105" y="180" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Memory safety</text>
  <text x="105" y="200" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Fearless concurrency</text>
  <text x="105" y="230" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="12" font-weight="bold">15.7 MB</text>

  <!-- Go -->
  <rect x="200" y="70" width="150" height="180" fill="#00ADD8" rx="8"/>
  <text x="275" y="100" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">Go</text>
  <text x="275" y="125" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Control Plane</text>
  <line x1="220" y1="140" x2="330" y2="140" stroke="#fff" stroke-opacity="0.3"/>
  <text x="275" y="160" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Fast compilation</text>
  <text x="275" y="180" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Built-in concurrency</text>
  <text x="275" y="200" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Ops-friendly</text>
  <text x="275" y="230" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="12" font-weight="bold">27.9 MB</text>

  <!-- Elixir -->
  <rect x="370" y="70" width="150" height="180" fill="#4E2A8E" rx="8"/>
  <text x="445" y="100" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">Elixir</text>
  <text x="445" y="125" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Query + MCP</text>
  <line x1="390" y1="140" x2="500" y2="140" stroke="#fff" stroke-opacity="0.3"/>
  <text x="445" y="160" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">BEAM reliability</text>
  <text x="445" y="180" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Real-time WebSocket</text>
  <text x="445" y="200" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Pattern matching</text>
  <text x="445" y="230" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="12" font-weight="bold">35.1 MB</text>

  <!-- TypeScript -->
  <rect x="540" y="70" width="150" height="180" fill="#3178c6" rx="8"/>
  <text x="615" y="100" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="16" font-weight="bold">TypeScript</text>
  <text x="615" y="125" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="11">Web Dashboard</text>
  <line x1="560" y1="140" x2="670" y2="140" stroke="#fff" stroke-opacity="0.3"/>
  <text x="615" y="160" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">React 19 + Next.js</text>
  <text x="615" y="180" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Type safety</text>
  <text x="615" y="200" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="10">Modern DX</text>
  <text x="615" y="230" text-anchor="middle" fill="#fff" font-family="system-ui" font-size="12" font-weight="bold">~50 MB</text>

  <!-- Total -->
  <text x="350" y="280" text-anchor="middle" fill="#4ecdc4" font-family="system-ui" font-size="16" font-weight="bold">
    Total Footprint: ~129 MB (vs 500MB+ typical)
  </text>
</svg>
```

---

## 5. Data for Charts (Python/Plotly)

```python
import plotly.graph_objects as go
import plotly.express as px

# Throughput comparison
throughput_data = {
    'System': ['AllSource', 'EventStoreDB', 'Kafka', 'Marten', 'Custom'],
    'Events/sec': [469000, 100000, 200000, 50000, 75000],
    'Color': ['#ce422b', '#666', '#666', '#666', '#444']
}

fig = go.Figure(data=[
    go.Bar(x=throughput_data['System'],
           y=throughput_data['Events/sec'],
           marker_color=throughput_data['Color'])
])
fig.update_layout(
    title='Event Throughput Comparison',
    yaxis_title='Events per Second',
    template='plotly_dark'
)
fig.write_image('throughput_comparison.png', scale=2)

# Latency comparison
latency_data = {
    'System': ['AllSource', 'EventStoreDB', 'Kafka', 'PostgreSQL'],
    'Latency_us': [11.9, 100, 500, 5000]
}

fig2 = go.Figure(data=[
    go.Bar(x=latency_data['System'],
           y=latency_data['Latency_us'],
           marker_color=['#4ecdc4', '#666', '#666', '#666'])
])
fig2.update_layout(
    title='Query Latency (p99) - Lower is Better',
    yaxis_title='Microseconds',
    yaxis_type='log',
    template='plotly_dark'
)
fig2.write_image('latency_comparison.png', scale=2)

# Container sizes
sizes = {
    'Service': ['Rust Core', 'Go Control', 'Elixir Query', 'Web Dashboard'],
    'Size_MB': [15.7, 27.9, 35.1, 50]
}

fig3 = px.pie(sizes, values='Size_MB', names='Service',
              color_discrete_sequence=['#ce422b', '#00ADD8', '#4E2A8E', '#3178c6'])
fig3.update_layout(title='Container Size Distribution (Total: 129MB)')
fig3.write_image('container_sizes.png', scale=2)
```

---

## 6. Export Instructions

### For PNG/SVG Export

1. **Mermaid diagrams**: Use `mmdc` CLI
   ```bash
   npm install -g @mermaid-js/mermaid-cli
   mmdc -i diagram.mmd -o diagram.png -t dark -b transparent
   ```

2. **SVG files**: Save as `.svg` and convert with Inkscape or ImageMagick
   ```bash
   inkscape --export-type=png --export-dpi=300 diagram.svg
   ```

3. **Python charts**: Install dependencies
   ```bash
   pip install plotly kaleido pandas
   ```

### For Presentations

- Export at 2x resolution for retina displays
- Use dark backgrounds for consistency
- Maintain brand colors across all assets
