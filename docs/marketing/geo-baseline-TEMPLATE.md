# GEO layer-3 baseline — TEMPLATE

> **AWAITING FIRST LIVE RUN — NO DATA YET.**
>
> This file is an empty template. There is **no baseline**: the layer-3 harness
> (prompt 025) was built in an environment with no LLM provider API keys, so no
> engine has ever been probed. Every table below is a placeholder.
>
> **Nothing in here may be filled in by hand.** Prompt 027 consumes this file as
> its experiment queue; a plausible-looking invented number is worse than an
> empty file, because nothing downstream can tell the difference.

## How to produce the real thing

```bash
# 1. Provide keys. An engine with no key is SKIPPED — loudly, and never as zero
#    share. ANTHROPIC_API_KEY also drives the layer-3b judge.
export OPENAI_API_KEY=...
export ANTHROPIC_API_KEY=...
export GEMINI_API_KEY=...
export PERPLEXITY_API_KEY=...
export ALLSOURCE_API_KEY=...          # to write the events to Core

# 2. Sweep. Three repetitions per prompt per engine is the floor.
cd tooling/geo && cargo build --release
./target/release/geo probe --repetitions 3 \
  --markdown-out ../../docs/marketing/geo-baseline-$(date -u +%F).md
```

That command **writes a dated file next to this one** — it does not overwrite
this template. Delete nothing here; this file stays as the shape a baseline
takes.

To re-render the same section later from the events already in Core (any
window, no LLM spend):

```bash
./target/release/geo report --since 30d \
  --markdown-out ../../docs/marketing/geo-baseline-$(date -u +%F).md
```

## What the generated file will contain

Section by section, all of it computed — none of it written by hand:

| section | contents |
|---|---|
| Engines skipped | any engine with no key, named, with the variable to set. Absence is not zero share. |
| Failed probes | probes that failed after retries. A partial sweep is kept, never discarded. |
| Layer 3a — share of voice | mention rate per intent class per engine, as **95% Wilson intervals**; mean reciprocal rank; competitor share on the same denominator; knowledge gaps (prompts every engine hedged on). |
| Layer 3b — interrogation accuracy | verdict counts per engine; every wrong claim quoted **verbatim** with the judge's reasoning and the repository file that defines the ground truth. |
| Source attribution | which hosts the engines cite, split ours vs third-party. |
| Observed vocabulary | the words the engines actually use, ranked by document frequency. Copy this into `geo-key-terms.md`. |
| Remediation backlog | wrong claims ordered by `frequency × severity`. Copy this into `geo-remediation-backlog.md`. |
| Spend | tokens and list cost per engine. Only Anthropic's rates are claimed; the rest report `unpriced`. |

## Reading rules that travel with the numbers

- **Share of voice alone is a vanity metric.** It says whether you appear in
  answers, not whether anyone buys anything. It is an input to layer 5 and a
  trend instrument — never the KPI.
- **Every rate carries a 95% Wilson interval.** Two cells whose intervals
  overlap have not moved relative to each other, however different the
  percentages look.
- **Do not publish a claimed relationship between a site change and a score over
  a window shorter than 12 weeks.**
- **A run is comparable to another only if the `run_id` digests match.** The
  digest is the hash of the frozen prompt set; editing the set starts a new
  baseline.

Full context: [`docs/runbooks/GEO_MEASUREMENT.md`](../runbooks/GEO_MEASUREMENT.md)
(Layer 3 section).
