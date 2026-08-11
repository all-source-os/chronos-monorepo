# GEO event contract (`geo.*`)

Source of truth for the durable Core events the GEO (Generative Engine
Optimization) measurement program emits for `www.all-source.xyz`.

An increasing share of AllSource's buyers will never see a blue link — they ask
ChatGPT, Claude, Perplexity or Gemini *"what should I use to give my agent
memory?"* and act on the answer. Before this contract there was no
instrumentation for that channel at all. Every layer of the measurement program
writes into this one queryable timeline instead of into five disconnected
scripts.

Producer today: [`tooling/geo`](../../../tooling/geo) (`geo-core` types +
emitter, `geo` CLI). The layer slices listed below fill in the callers.

## Why a contract, not just types

`geo-core` is the only producer *today*, but the report reader, the
optimization loop and anything that later reads this stream out of Core are
separate consumers with their own release cadence. The contract is the
authority; the Rust types in
[`tooling/geo/geo-core/src/events.rs`](../../../tooling/geo/geo-core/src/events.rs)
are derived from it, never the reverse.

Drift is mechanically prevented: the JSON in [`examples/`](examples/) is
**generated from the emitter**, and
[`geo-core/tests/contract_examples.rs`](../../../tooling/geo/geo-core/tests/contract_examples.rs)
fails if a committed example and the real serialisation disagree. Regenerate
after changing a payload type:

```bash
cd tooling/geo && cargo test -p geo-core -- --ignored regenerate_contract_examples
```

This is deliberate. gh#250 burned us with mocks that faithfully honoured a
contract the server did not implement — so the fixtures here are the emitter's
own output, not a hand-typed approximation of it.

## Envelope

Every `geo.*` event is POSTed to `POST /api/v1/events` **through the Control
Plane gateway** (`https://api.all-source.xyz`). Core does not authenticate
public traffic; the gateway validates the key, injects `tenant_id` and
forwards.

| field | who sets it | rule |
|---|---|---|
| `event_type` | emitter | const — one of the seven below |
| `entity_id` | emitter | the dedupe / lifecycle key (see below) |
| `payload` | emitter | per-event (see the tables below) |
| `metadata` | emitter | `{"emitter": "tooling/geo", "idempotency_key": "<hex>"}` |
| `tenant_id` | Control Plane | injected at the gateway; **never client-trusted** |

Core assigns `id`, `timestamp` and the per-entity monotonic `version`
server-side — they are not part of the ingest body. Core is the database (WAL +
Parquet + DashMap): these events are durable across restarts, which is what
makes a 12-week trend window possible at all.

### `entity_id`

Every GEO entity id is prefixed `geo:` so GEO telemetry is trivially separable
from — and can never be confused with — a customer tenant's entity stream.

- **Observations** (`referral`, `crawl`, `sov`, `interrogation`, `selfreport`)
  → `geo:<namespace>:<idempotency_key>`. One entity per observation.
- **Experiments** (`started`, `scored`) → `geo:experiment:<experiment_id>`.
  One entity per experiment, so its lifecycle is one ordered stream and
  `?entity_id=geo:experiment:<id>&order=desc&limit=1` yields current state.

### Idempotency

Each event type has a *natural key* — the tuple of facts that makes it the same
observation rather than a new one. `geo-core` hashes that tuple (SHA-256,
first 128 bits, hex) into `metadata.idempotency_key`, which is also the
`entity_id` suffix for observations.

Because the key lands in the `entity_id`, **re-ingesting the same log window
appends a version to the same Core entity instead of creating a second one**.
Counting distinct entities is therefore replay-safe; counting raw events is
not. `geo report` prints both columns for exactly this reason.

Timestamps that feed a key are truncated to whole seconds, so sub-second jitter
between two reads of the same log line cannot split one observation into two.
Payloads keep full precision.

| event | natural key |
|---|---|
| `geo.referral.observed` | `observed_at` (s) + `surface` + `landing_path` + `session_id` + `referrer_url` |
| `geo.crawl.observed` | `observed_at` (s) + `bot` + `path` + `status` + `source` |
| `geo.sov.probed` | `run_id` + `engine` + `prompt_id` |
| `geo.interrogation.probed` | `run_id` + `engine` + `prompt_id` + `claim_id` |
| `geo.selfreport.captured` | `observed_at` (s) + `source` + `surface` + `contact_ref` |
| `geo.experiment.started` | `experiment_id` + `iteration` + `"started"` |
| `geo.experiment.scored` | `experiment_id` + `iteration` + `metric` + `"scored"` |

### `schema_version`

Every payload carries `schema_version` (currently `1`). Later slices add fields
**additively** at version 1. Bump only when an existing field changes meaning
or type, so a replay over a mixed-version stream can branch on it.

### Time

Every timestamp is UTC, RFC 3339. GEO is trend analysis over multi-week
windows and a timezone slip silently ruins one, so nothing in this contract is
local time. The CLI accepts an offset (`+02:00`) and normalises it; it refuses
a timestamp with no offset at all.

## Events

| event | entity_id | meaning | layer | produced by | example |
|---|---|---|---|---|---|
| `geo.referral.observed` | `geo:referral:<key>` | a human session arriving from an AI surface | 1 | prompt 026 | [ex](examples/geo.referral.observed.json) |
| `geo.crawl.observed` | `geo:crawl:<key>` | a verified AI bot hit | 2 | prompt 024 | [ex](examples/geo.crawl.observed.json) |
| `geo.sov.probed` | `geo:sov:<key>` | one scored share-of-voice probe result | 3a | prompt 025 | [ex](examples/geo.sov.probed.json) |
| `geo.interrogation.probed` | `geo:interrogation:<key>` | one scored brand-accuracy probe result | 3b | prompt 025 | [ex](examples/geo.interrogation.probed.json) |
| `geo.selfreport.captured` | `geo:selfreport:<key>` | a human telling us an AI sent them | 4 | prompt 026 | [ex](examples/geo.selfreport.captured.json) |
| `geo.experiment.started` | `geo:experiment:<id>` | one optimization iteration opened | 5 | prompt 027 | [ex](examples/geo.experiment.started.json) |
| `geo.experiment.scored` | `geo:experiment:<id>` | one optimization iteration scored | 5 | prompt 027 | [ex](examples/geo.experiment.scored.json) |

The prompt column is the plan of record at time of writing. If a slice lands
under a different number, fix it here **and** in the `LAYERS` table in
[`geo-cli/src/report.rs`](../../../tooling/geo/geo-cli/src/report.rs) — the two
are the only places the layer map exists.

---

### `geo.referral.observed` — layer 1

A human session arriving from an AI surface.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `observed_at` | string (RFC 3339, UTC) | yes | when the session was observed |
| `surface` | string | yes | AI surface, free-form host-ish id (`"chatgpt.com"`). **Not** an enum — the surface taxonomy is the producing slice's job |
| `referrer_url` | string \| null | yes | full referrer when the surface sent one |
| `landing_path` | string | yes | site-relative path landed on |
| `session_id` | string \| null | yes | opaque analytics session id |
| `user_agent` | string \| null | yes | arriving browser's user agent |

[`examples/geo.referral.observed.json`](examples/geo.referral.observed.json)

---

### `geo.crawl.observed` — layer 2

One verified AI bot hit.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `observed_at` | string (RFC 3339, UTC) | yes | when the request hit the edge |
| `bot` | string | yes | normalised bot id (`"gptbot"`). The bot **taxonomy** — families, owners, train-vs-retrieve — belongs to prompt 024, not here |
| `verified` | boolean | yes | whether the claimed identity was verified (reverse DNS / IP range). The verification **method** belongs to prompt 024; this contract records only the verdict |
| `user_agent` | string | yes | raw `User-Agent` header |
| `path` | string | yes | site-relative path requested |
| `status` | integer | yes | HTTP status served |
| `source` | string | yes | which log/edge produced the line (`"vercel-log-drain"`) |

[`examples/geo.crawl.observed.json`](examples/geo.crawl.observed.json)

---

### `geo.sov.probed` — layer 3a

One scored share-of-voice probe result.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `observed_at` | string (RFC 3339, UTC) | yes | when the answer came back |
| `run_id` | string | yes | groups every probe in one sweep |
| `engine` | string | yes | `"chatgpt"`, `"claude"`, `"perplexity"`, `"gemini"` |
| `prompt_id` | string | yes | stable id within the probe set. The **set** belongs to prompt 025 |
| `prompt_text` | string | yes | the prompt as sent, kept verbatim so a historical score stays readable after the set is edited |
| `mentioned` | boolean | yes | was AllSource named at all |
| `rank` | integer \| null | yes | 1-based position among named products; `null` when absent |
| `competitors` | array of string | yes | other products named, in order of appearance |
| `cited_urls` | array of string | yes | URLs the engine cited |
| `score` | number | yes | normalised 0.0–1.0. The **formula** belongs to prompt 025; this contract reserves only the slot and the range |

[`examples/geo.sov.probed.json`](examples/geo.sov.probed.json)

---

### `geo.interrogation.probed` — layer 3b

One scored brand-accuracy probe result: not *whether* we were named, but
whether what was said about us is true.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `observed_at` | string (RFC 3339, UTC) | yes | when the answer came back |
| `run_id` | string | yes | groups every probe in one sweep |
| `engine` | string | yes | engine probed |
| `prompt_id` | string | yes | stable id within the probe set |
| `prompt_text` | string | yes | the prompt as sent |
| `claim_id` | string | yes | which factual claim was under test (`"pricing"`, `"durability"`, `"license"`) |
| `verdict` | string | yes | free-form verdict. The allowed vocabulary is fixed by prompt 025; this contract guarantees only that the field exists |
| `answer_excerpt` | string | yes | the part of the answer the verdict was drawn from |
| `cited_urls` | array of string | yes | URLs the engine cited |
| `score` | number | yes | normalised 0.0–1.0 |

[`examples/geo.interrogation.probed.json`](examples/geo.interrogation.probed.json)

---

### `geo.selfreport.captured` — layer 4

A human telling us an AI sent them — the only layer that survives a referrer
being stripped.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `observed_at` | string (RFC 3339, UTC) | yes | when the human told us |
| `source` | string | yes | where the answer was collected (`"signup-form"`) |
| `surface` | string | yes | what the human said sent them (`"ChatGPT"`) |
| `verbatim` | string \| null | yes | full free-text answer when the question allowed one |
| `contact_ref` | string \| null | yes | opaque reference back to the person (tenant id, hashed handle) |

**Privacy:** `contact_ref` is never a raw email address. GEO telemetry is a
trend timeline, not a place to accumulate PII. Do not extend this payload with
identifying fields.

[`examples/geo.selfreport.captured.json`](examples/geo.selfreport.captured.json)

---

### `geo.experiment.started` — layer 5

One optimization iteration opened. Shares an entity with its `scored` event.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `started_at` | string (RFC 3339, UTC) | yes | when the iteration was opened |
| `experiment_id` | string | yes | stable id, shared by every event in the lifecycle |
| `iteration` | integer | yes | iteration within the experiment, from 1 |
| `hypothesis` | string | yes | what we think the change will do, in one sentence |
| `target_layer` | string | yes | which layer the hypothesis aims at (`"referral"`, `"crawl"`, `"sov"`, `"interrogation"`, `"selfreport"`) |
| `changes` | array of string | yes | what was actually changed — paths, surfaces, files |
| `baseline_score` | number \| null | yes | target metric before the change, when known |

[`examples/geo.experiment.started.json`](examples/geo.experiment.started.json)

---

### `geo.experiment.scored` — layer 5

One optimization iteration scored. This is the single metric the optimization
loop reads back out of Core.

| field | type | required | meaning |
|---|---|---|---|
| `schema_version` | integer | yes | `1` |
| `scored_at` | string (RFC 3339, UTC) | yes | when the score was computed |
| `experiment_id` | string | yes | the experiment this scores |
| `iteration` | integer | yes | iteration scored |
| `metric` | string | yes | which metric was scored (`"sov.mention_rate"`) |
| `score` | number | yes | the score |
| `window_start` | string (RFC 3339, UTC) | yes | start of the measurement window, inclusive |
| `window_end` | string (RFC 3339, UTC) | yes | end of the measurement window, exclusive |
| `verdict` | string | yes | what the loop decided. Vocabulary belongs to prompt 027 |
| `notes` | string \| null | yes | anything a human should read before trusting the verdict |

[`examples/geo.experiment.scored.json`](examples/geo.experiment.scored.json)

## Reading the family back

```
GET /api/v1/events/query?event_type_prefix=geo.&since=<rfc3339>&until=<rfc3339>
```

`geo report` does exactly this and tallies per layer — see
[`docs/runbooks/GEO_MEASUREMENT.md`](../../runbooks/GEO_MEASUREMENT.md).

## Adding an event

1. Add the variant to `GeoEventType` and a payload struct in `geo-core`.
2. Add its natural key to `GeoEvent::idempotency_key`.
3. Add a canonical example to `geo-core/src/samples.rs`.
4. Put it in a layer in `geo-cli/src/report.rs`.
5. Regenerate `examples/` (command at the top of this file).
6. Document it here.

Steps 3 and 5 are load-bearing: `the_examples_directory_holds_exactly_the_contract`
fails if `examples/` and `GeoEventType::ALL` disagree, so an event cannot be
added on one side only.
