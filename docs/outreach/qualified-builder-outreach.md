# Qualified builder outreach ledger

Purpose: prevent duplicate founder outreach and retain public evidence, exact
drafts, approval state, and send outcomes.

Rules:

- Public professional evidence and channels only.
- Never use personal email, private groups, scraped contact data, or sensitive
  information.
- Do not send before founder approves exact batch.
- One initial message per builder. Follow up only when explicitly marked due.
- No request for review, endorsement, testimonial, or public mention.

## Deduplication index

| Builder / handle | Project | First prepared | Channel | Status | Follow-up due |
| --- | --- | --- | --- | --- | --- |
| Sajjad Khan | LangGraph crash recovery | 2026-08-28 | Website contact form | Blocked: sender-data approval required | No |
| Doniyor Aliyev | LangGraph production chat | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:32 WEST | 2026-09-03 |
| Kiell Tampubolon | DurableClaimStore proposal | 2026-08-28 | DEV Connect | Blocked: DEV authentication required | No |
| Diogo Santos | ContextWeaver | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:36 WEST | 2026-09-03 |
| Sanjay Rohith L | Dejavu | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:39 WEST | 2026-09-03 |
| Dineshsuriya D | AutoGen GraphFlow | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:41 WEST | 2026-09-03 |
| Israel “Izzy” Ekpo | PydanticAI remote persistence | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:42 WEST | 2026-09-03 |
| Tathastu Naranje | CrewAI negotiation rehearsal | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:43 WEST | 2026-09-03 |
| Nithin R | PydanticAI session persistence | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:44 WEST | 2026-09-03 |
| Matthias Howell | CrewAI distributed memory | 2026-08-28 | LinkedIn | Sent 2026-08-28 02:45 WEST | 2026-09-03 |
| Thomas Connally / `tcconnally` | Perseus Vault and Ledger | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Morrow / `agent-morrow` | compression-monitor | 2026-08-28 | Bluesky | Awaiting batch approval | No |
| Mohamed Amine Ferhi / `ferhimedamine` | DakeraStorage | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Zelal Helin Akdoğan / `helinakdogan` | Agent Magnet | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Aleksey Safonov / `safal207` | Causal-Memory-Layer | 2026-08-28 | X | Awaiting batch approval | No |
| David SF / `dsfaccini` | PydanticAI suspended-turn resume | 2026-08-28 | X | Awaiting batch approval | No |
| Kázmér Nagy-Betegh / `kazmer97` | PydanticAI durable Code Interpreter history | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Douwe Maan / `DouweM` | PydanticAI compaction history | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Arjun Ganesh / `iarjunganesh` | Google ADK RemoteA2A pipeline | 2026-08-28 | LinkedIn | Awaiting batch approval | No |
| Brian Kilgore / `briankilgore` | Agno bookkeeper team | 2026-08-28 | LinkedIn | Awaiting batch approval | No |

Prior batch detail and verified send log:
[`docs/marketing/2026-08-28-qualified-builder-outreach.md`](../marketing/2026-08-28-qualified-builder-outreach.md).

## Batch — 2026-08-28 08:00 UTC

Batch status: **awaiting founder approval; nothing sent**.

### 1. Thomas Connally — Perseus Vault and Ledger

- Builder: Thomas Connally (`tcconnally`), founder of Perseus Computing.
- Public source: [Perseus platform](https://perseus.observer/) and
  [Mimir MCP showcase](https://github.com/modelcontextprotocol/ext-apps/issues/692).
- Exact public evidence: Perseus separates current context, time-valid durable
  memory, and hash-chained evidence. Thomas reports Mimir storing cross-session
  entities, bitemporal history, hybrid recall, and encrypted local state for
  MCP agents.
- Inferred memory problem: correction and recall layers still need a shared,
  testable answer for which fact version reached an agent before action.
- Fit: active founder already running local agent-memory integrations; direct
  overlap with AllSource provenance and historical reconstruction.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/thomas-connally>

Message (88 words):

> Hi Thomas — Perseus separates live context, Vault memory, and Ledger evidence,
> while Mimir keeps bitemporal facts across agent sessions. I build AllSource,
> an event-sourced memory and provenance layer for agents. I’d like to test one
> bounded flow: record a decision, correct it later, then reconstruct which
> version an agent saw before acting. I can offer direct integration help plus
> design-partner access. Interested in comparing immutable event history with
> Vault’s hybrid recall on that flow? If this is not relevant, reply “pass” and
> I will not follow up.

### 2. Morrow — compression-monitor

- Builder: Morrow (`agent-morrow`), persistent production agent.
- Public source: [compression-monitor proposal](https://github.com/crewAIInc/crewAI/issues/5155)
  and [Morrow project page](https://morrow.run/).
- Exact public evidence: compression-monitor measures ghost-lexicon decay,
  semantic drift, and tool-call sequence shift across context boundaries. A
  live production session recorded 88.1% vocabulary decay after compaction.
- Inferred memory problem: behavioral drift can remain invisible unless a
  boundary measurement can be joined to exact pre- and post-compaction state.
- Fit: production agent, active memory instrumentation, and reproducible
  cross-session drift signal.
- Channel: Bluesky direct message or public reply.
- Destination: <https://bsky.app/profile/morrow00.bsky.social>

Message (86 words):

> Hi Morrow — compression-monitor’s ghost-lexicon decay and tool-sequence shift
> target a failure most systems miss: an agent stays operational after context
> rotation while its behavioral state changes. I build AllSource, focused on
> durable, source-linked agent history. I’d like to pair your pre/post-boundary
> probes with a replayable event trail, so a drift spike can be tied to exact
> memories and decisions present before compression. I can offer hands-on
> integration and design-partner access. Worth testing one live boundary? If
> not, reply “pass” and I will close the loop.

### 3. Mohamed Amine Ferhi — DakeraStorage

- Builder: Mohamed Amine Ferhi (`ferhimedamine`), founder of Dakera AI.
- Public source: [CrewAI DakeraStorage integration](https://github.com/crewAIInc/crewAI/issues/6409).
- Exact public evidence: DakeraStorage replaces process-bound LanceDB with a
  self-hosted REST backend that persists across restarts and container rebuilds,
  supports shared agent instances, decay-weighted recall, and inspection.
- Inferred memory problem: decayed retrieval alone cannot show which corrected
  fact version was current or which source caused a change.
- Fit: active persistence backend for CrewAI with direct cross-process and
  recall requirements.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/medamineferhi>

Message (84 words):

> Hi Mohamed — DakeraStorage moves CrewAI memory off local LanceDB so multiple
> crews can share it across container rebuilds, then applies decay-weighted
> recall. I build AllSource, an event-sourced agent-memory and provenance layer.
> One useful test would compare decayed recall with immutable correction
> history: which fact was retrieved, which version was current, and which source
> event changed it. I can help wire the test and provide design-partner access.
> Interested in a small DakeraStorage experiment? If not relevant, reply “pass”
> and I will not follow up.

### 4. Zelal Helin Akdoğan — Agent Magnet

- Builder: Zelal Helin Akdoğan (`helinakdogan`), Agent Magnet founder and AI
  engineer.
- Public source: [Agent Magnet CrewAI integration](https://github.com/crewAIInc/crewAI/issues/6050).
- Exact public evidence: Agent Magnet combines Redis behavioral memory, Qdrant
  episodic memory, and Neo4j knowledge. It learns from corrections, rejections,
  and implicit behavior rather than restarting each CrewAI session from zero.
- Inferred memory problem: learned preferences need supersession history so an
  agent can distinguish current preference from stale behavior and explain why.
- Fit: serious multi-layer memory product with explicit cross-session learning.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/helinakdogan/>

Message (87 words):

> Hi Helin — Agent Magnet’s Redis, Qdrant, and Neo4j layers learn from
> corrections, rejections, and implicit behavior across sessions. I build
> AllSource, focused on durable decision history and provenance for agents. I’d
> like to test a sharp edge: when observed behavior changes, can the agent
> distinguish the latest preference from a superseded one and reconstruct why
> its profile changed? I can offer hands-on integration help and design-partner
> access for one behavioral-memory flow. Would that help Agent Magnet? If not,
> reply “pass” and I will leave it there.

### 5. Aleksey Safonov — Causal-Memory-Layer

- Builder: Aleksey Safonov (`safal207`), independent AI-safety builder.
- Public source: [Causal-Memory-Layer CrewAI integration](https://github.com/crewAIInc/crewAI/issues/6063)
  and [project repository](https://github.com/safal207/Causal-Memory-Layer).
- Exact public evidence: Causal-Memory-Layer validates whether an agent or tool
  action retains a parent cause, approval identifier, and responsibility
  lineage; repository was active on 2026-08-27.
- Inferred memory problem: lineage validation needs durable original and
  superseding events so correction does not erase prior evidence.
- Fit: active agent accountability project whose core object is causal,
  replayable action history.
- Channel: X direct message or public reply.
- Destination: <https://x.com/lim746048>

Message (84 words):

> Hi Aleksey — Causal-Memory-Layer checks whether a completed action still has a
> valid parent cause, approval, and responsibility lineage. I build AllSource,
> an event-sourced memory and provenance layer for agents. A strong joint test
> would record action, approval, correction, and replay as immutable events,
> then ask CML to flag lineage that became missing or superseded without losing
> original evidence. I can offer direct engineering help and design-partner
> access. Interested in testing one CrewAI-style trace? If not, reply “pass” and
> I will not follow up.

### 6. David SF — PydanticAI suspended-turn resume

- Builder: David SF (`dsfaccini`), AI developer at alecs.
- Public source: [PydanticAI suspended-turn issue](https://github.com/pydantic/pydantic-ai/issues/7802).
- Exact public evidence: his deterministic reproduction shows resume failing
  when durable history contains a part transformed by `prepare_messages`; the
  continuation path replays a different seed. Impact includes OpenAI background
  mode and Anthropic `pause_turn` after output retry.
- Inferred memory problem: durable execution cannot reconstruct exact provider
  continuation state when prepared and recorded history diverge.
- Fit: recent, reproducible cross-run state failure on a serious agent path.
- Channel: X direct message.
- Destination: <https://x.com/dasfacc>

Message (87 words):

> Hi David — your PydanticAI #7802 repro shows a suspended turn becoming
> unresumable when prepared history differs from the continuation seed actually
> replayed, especially after tool-availability or output-retry parts. I build
> AllSource, an event-sourced memory and provenance layer for agents. I’d like
> to test storing pre-wire history transformations as explicit events, so
> resume reconstructs exact continuation state instead of an inferred snapshot.
> I can help build an adapter and offer design-partner access. Worth testing one
> background-mode flow? If not, reply “pass” and I will close the loop.

### 7. Kázmér Nagy-Betegh — durable Code Interpreter history

- Builder: Kázmér Nagy-Betegh (`kazmer97`), AWS engineer building with
  PydanticAI.
- Public source: [expired Code Interpreter history issue](https://github.com/pydantic/pydantic-ai/issues/7461).
- Exact public evidence: durable `message_history` replays an expired OpenAI
  Code Interpreter `container_id`; each later run receives the same 400 and
  remains broken until only provider-bound state is removed and repaired history
  is persisted.
- Inferred memory problem: provider state needs explicit invalidation history
  without deleting unrelated conversation evidence.
- Fit: observed provider failure plus deterministic public reproduction of a
  permanently poisoned durable session.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/kazmer97>

Message (89 words):

> Hi Kázmér — your PydanticAI #7461 case is nasty: durable history keeps
> replaying an expired Code Interpreter container ID, so every future turn fails
> until provider-bound state is surgically removed. I build AllSource, focused
> on durable, source-linked agent history. I’d like to test an invalidation
> event that retires the expired container while preserving unrelated calls,
> then records which repaired history version resumed successfully. I can
> provide direct integration help and design-partner access. Interested in a
> minimal recovery adapter? If not relevant, reply “pass” and I will not follow
> up.

### 8. Douwe Maan — PydanticAI compaction history

- Builder: Douwe Maan (`DouweM`), lead PydanticAI maintainer.
- Public source: [empty CompactionPart issue](https://github.com/pydantic/pydantic-ai/issues/7773).
- Exact public evidence: an empty compaction summary is treated as a wire
  boundary and discards all earlier messages even though it cannot replace
  them; a later tool call can then fail its evidence check.
- Inferred memory problem: compaction requires a verifiable replacement event
  before old history can be hidden from provider replay.
- Fit: core framework maintainer working directly on durable-execution and
  history semantics.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/douwem>

Message (92 words):

> Hi Douwe — PydanticAI #7773 shows an empty CompactionPart being accepted as a
> wire boundary, dropping history it cannot replace and potentially orphaning a
> later tool call from its evidence. I build AllSource, an event-sourced memory
> and provenance layer for agents. I’d like to test a compacted-history contract
> where replacement, source range, and validity are explicit events before any
> prior messages become hidden. I can offer engineering help and design-partner
> access around one provider flow. Useful to compare with PydanticAI’s
> compaction semantics? If not, reply “pass” and I will leave it there.

### 9. Arjun Ganesh — Google ADK RemoteA2A pipeline

- Builder: Arjun Ganesh (`iarjunganesh`), senior distributed-systems engineer
  building audit-trailed agentic AI for regulated workflows.
- Public source: [Google ADK RemoteA2A state issue](https://github.com/google/adk-python/issues/6854).
- Exact public evidence: his production `SequentialAgent` pipeline worked
  locally, but two `RemoteA2aAgent` boundaries lost `output_key` and state-only
  `state_delta`; the next step used earlier plausible content. Unit tests did
  not reveal the transport-boundary failure.
- Inferred memory problem: remote handoffs lack an acknowledged, reconstructable
  state transition shared by caller and worker.
- Fit: production multi-agent pipeline with audit and citation requirements.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/iarjunganesh>

Message (91 words):

> Hi Arjun — your ADK #6854 report shows a SequentialAgent pipeline working
> in-process for weeks, then silently losing output_key and state_delta across a
> RemoteA2aAgent boundary and acting on earlier plausible content. I build
> AllSource, an event-sourced memory and provenance layer for agents. I’d like
> to test explicit handoff events with source, version, and acknowledgement, so
> caller and remote agent can reconstruct the same state across A2A. I can offer
> direct engineering help and design-partner access. Worth testing on a
> sanitized pipeline? If not, reply “pass” and I will leave it there.

### 10. Brian Kilgore — Agno bookkeeper team

- Builder: Brian Kilgore (`briankilgore`), SaaS builder running coordinated AI
  agent teams.
- Public source: [Agno team-member persistence issue](https://github.com/agno-agi/agno/issues/9339)
  and [public agent-team post](https://www.linkedin.com/in/brianleekilgore).
- Exact public evidence: his Agno 2.8.3 reproduction uses a persistent
  `bookkeeper` member with Postgres/Supabase. Delegated runs stay only in team
  history, create no member session, and capture no member-attributed memory;
  direct runs persist correctly.
- Inferred memory problem: specialist identity and memory cannot compound when
  direct and delegated invocation write different histories.
- Fit: serious multi-agent system, deterministic control case, and explicit
  request for durable per-member memory.
- Channel: LinkedIn direct message.
- Destination: <https://www.linkedin.com/in/brianleekilgore>

Message (91 words):

> Hi Brian — your Agno #9339 repro shows the bookkeeper retaining memory when
> run directly, but losing its own session and agent-attributed memory when the
> team delegates to it. I build AllSource, an event-sourced memory and
> provenance layer for agents. I’d like to test one ledger where direct and
> delegated runs share specialist identity while preserving team context and
> invocation source. I can offer hands-on integration help and design-partner
> access for the Postgres flow. Would that test help your agent team? If not,
> reply “pass” and I will not follow up.

## Approval and send log

- [ ] Founder approves exact ten recipients and exact drafts.
- [ ] Send approved messages one at a time through authenticated channels.
- [ ] Verify each successful send and record canonical URL, timestamp, and
  outcome below.

| Sent time | Recipient | Channel | Public URL | Outcome |
| --- | --- | --- | --- | --- |
| — | — | — | — | Awaiting approval |
