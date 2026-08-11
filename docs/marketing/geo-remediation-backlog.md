# GEO remediation backlog — wrong claims the engines make about us

> **AWAITING FIRST LIVE RUN — NO DATA YET.**
>
> This file is empty on purpose. The layer-3 harness (prompt 025) was built in
> an environment with no LLM provider API keys, so **no engine has ever been
> asked anything about AllSource** and no claim has been graded.
>
> **Do not add rows by hand.** Prompt 027 consumes this file as its experiment
> queue and will spend real work on whatever is in it. An invented row sends
> that work at a problem that may not exist, and nothing downstream can tell an
> invented row from an observed one.

## The one command that fills it

```bash
export OPENAI_API_KEY=... ANTHROPIC_API_KEY=... GEMINI_API_KEY=... PERPLEXITY_API_KEY=...
cd tooling/geo && cargo build --release
./target/release/geo probe --family interrogation --repetitions 3 \
  --markdown-out ../../docs/marketing/geo-baseline-$(date -u +%F).md
```

`ANTHROPIC_API_KEY` is required twice over: once for the `claude` engine and
once for the LLM-as-judge. Without it every claim comes back `unscored` and this
backlog stays empty — correctly, because nothing was graded.

Then copy the **Remediation backlog** table from the generated baseline into the
table below.

## Ordering

`frequency × severity`, where frequency is the number of answers that got the
claim wrong and severity is declared per claim in
`tooling/geo/prompts/interrogation.toml`:

| severity | weight | means |
|---|---|---|
| critical | 8 | a wrong answer directly costs a sale or a trust relationship |
| high | 4 | a wrong answer sends a qualified buyer to a competitor |
| medium | 2 | a correctable misunderstanding |
| low | 1 | cosmetic |

Two rules that shape what lands here:

- **`absent` is not in this backlog.** A model that says nothing about our
  licence has not got it wrong. Silence is a content gap (write the page);
  wrongness is a correction (fix the page). They are different work and they are
  tracked separately.
- **A third-party page carrying a wrong claim is the highest-value target in the
  programme.** We cannot edit it, so it has to be corrected at source or
  out-published. The generated table names the third-party hosts cited in the
  answers that got a claim wrong — that is the only lead you get.

## Backlog

_Empty — no live sweep has run._

| # | claim | severity | wrong in | engines | score | fix (file that defines the truth) | likely third-party source |
|---|---|---|---|---|---|---|---|
| _(none yet)_ | | | | | | | |

## Already known, without a probe

These are contradictions **inside this repository**, found while grounding the
interrogation set's 19 ground-truth claims. They are not in the backlog table
above — that table is reserved for what a live run observes — but several of
them are almost certainly *causes* of the wrong answers a live run will find,
and one of them is the cheapest fix in the programme:

1. **`apps/web/public/llms.txt` is the most out-of-date surface we have, and it
   is the file we serve specifically to LLMs.** It still advertises retired
   Free/Pro/Growth tiers, `$29`/`$79` prices, a 100,000-event free tier, and
   "43 MCP tools". One commit fixes all of it.
2. **Licence split.** Root `LICENSE` is Apache-2.0 and `LICENSE-BSL` covers
   enterprise features, but `apps/core/LICENSE` and the published SDK manifests
   still say MIT.
3. **MCP tool count** is published as 73 (site, README), 61 (README badge) and
   43 (llms.txt).
4. **Currency** is settled as GBP in launch copy while `apps/web/src/lib/config.ts`
   carries USD-shaped fallback strings.
5. **Latency** is 11.9µs p99, but at least two of our own surfaces say
   "sub-microsecond" — an order of magnitude out, and something we would
   otherwise blame a model for repeating.

Full detail with citations: [`docs/runbooks/GEO_MEASUREMENT.md`](../runbooks/GEO_MEASUREMENT.md)
→ "Ground-truth contradictions".
