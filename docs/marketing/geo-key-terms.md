# Observed model vocabulary — the words the engines actually use

> **AWAITING FIRST LIVE RUN — NO DATA YET.**
>
> This file is empty on purpose. The layer-3 harness (prompt 025) was built in
> an environment with no LLM provider API keys, so no engine has ever been
> probed and **not one term has been observed**.
>
> **Do not write terms in here from intuition, from our own marketing copy, or
> from a fixture run.** The entire value of this file is that it records
> language we *observed* rather than language we chose; a hand-written row
> destroys that and is indistinguishable from a real one to prompt 027, which
> consumes this file.

## The one command that fills it

```bash
export OPENAI_API_KEY=... ANTHROPIC_API_KEY=... GEMINI_API_KEY=... PERPLEXITY_API_KEY=...
cd tooling/geo && cargo build --release
./target/release/geo probe --family sov --repetitions 3 \
  --markdown-out ../../docs/marketing/geo-baseline-$(date -u +%F).md
```

Then copy the **Observed vocabulary** table from that generated file into the
table below, with the run id and date it came from.

Vocabulary can only be extracted from a *fresh sweep*: the stored
`geo.sov.probed` payload keeps the score and the citations, not the whole answer
text, so `geo report` cannot reconstruct it from Core.

## Why this file exists

Our marketing vocabulary is not the models' vocabulary. We say "AI-native event
store for temporal data intelligence"; the engines answering our buyers' actual
questions may say "memory layer", "persistence", "state store", "context
management", or something nobody in this repository has ever typed. **You cannot
optimise for language you have not observed.** This ranked map is the direct
input to prompt 027's content work.

It is deliberately **not** filtered against our own copy. The overlap — or the
absence of it — is the finding.

## Method (so a later run is comparable)

- Terms are 1–3 word n-grams from SOV answer text only (family `sov`).
- Ranked by **document frequency**: how many distinct answers contained the
  term, not how many times it occurred. One model repeating itself is not
  evidence of vocabulary.
- A term must appear in at least **2 answers** to be listed at all.
- N-grams that start or end on a stopword are dropped as fragments.
- Product names are kept and flagged rather than removed — how often the engines
  reach for a competitor's name *is* vocabulary.

Implementation: `tooling/geo/geo-core/src/scoring.rs` (`key_terms`).

## Observed terms

_Empty — no live sweep has run._

| term | answers containing it | engines | product name? | first seen (run id) |
|---|---|---|---|---|
| _(none yet)_ | | | | |
