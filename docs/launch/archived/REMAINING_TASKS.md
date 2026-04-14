# Remaining Tasks — Post-Launch

All tasks are currently **deferred**. Complete as needed post-launch.

---

## P0-003: Add SLA Monitoring and Alerting [P1]

> Deferred until post-launch — needs real users and SLOs first.

- [ ] Define SLOs (latency p99, uptime %, error rate thresholds)
- [ ] Implement health/metrics monitoring
- [ ] Add alerting (PagerDuty, Slack, etc.)
- [ ] Dashboard for SLA tracking

---

## SALES-001: Export C4 Diagrams to PNG/SVG [P3]

- [ ] Install mermaid-cli: `npm install -g @mermaid-js/mermaid-cli`
- [ ] Export all Mermaid diagrams from `C4_ARCHITECTURE_MERMAID.md` to PNG
- [ ] Export with dark theme and 2x resolution
- [ ] Install Structurizr CLI for `chronos.dsl` export
- [ ] Export Structurizr diagrams to PlantUML and PNG
- [ ] Save all exports to `docs/sales/diagrams/exports/`
- [ ] Verify diagrams render correctly

---

## SALES-002: Generate Performance Charts with Python [P3]

- [ ] Install dependencies: `pip install plotly kaleido pandas`
- [ ] Generate throughput comparison chart (Chronos vs competitors)
- [ ] Generate latency comparison chart
- [ ] Generate container size pie chart
- [ ] Export all charts as PNG at 2x resolution
- [ ] Save to `docs/sales/pitch-deck/charts/`
- [ ] Verify charts match brand colors

---

## SALES-003: Convert Pitch Deck to Presentation Format [P3]

> Blocked by SALES-002 (needs charts first).

- [ ] Choose format: Google Slides, Keynote, or Reveal.js
- [ ] Create 14 slides from `SALES_PITCH_DECK.md`
- [ ] Add visual assets and charts
- [ ] Add speaker notes
- [ ] Create PDF export for sharing
- [ ] Test presentation flow (15-20 min timing)
- [ ] Save to `docs/sales/pitch-deck/`
