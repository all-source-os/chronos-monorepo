//! `geo probe` — run the frozen probe set across the engines, score it, judge
//! it, and write the result to Core as `geo.sov.probed` / `geo.interrogation.probed`.
//!
//! ## Shape of a sweep
//!
//! ```text
//! for each family (sov, interrogation)
//!   for each repetition 1..=N        <- N ≥ 3; this is what makes it a measurement
//!     for each engine                 <- in parallel, bounded per engine
//!       for each prompt               <- in parallel, bounded per engine
//! ```
//!
//! Repetitions are the whole point. One sample per prompt tells you what one
//! roll of the dice said; the report's intervals are only meaningful because
//! there is more than one. Each repetition gets its own `run_id`, so the N
//! samples are N Core entities rather than N versions of one — see
//! [`geo_core::prompts::PromptSet::run_id`].
//!
//! ## Failure policy
//!
//! Nothing here aborts a sweep. A missing key skips an engine loudly; a probe
//! that fails after retries is recorded as a failure and the rest continue; a
//! judge that cannot be reached marks its claims `unscored` rather than
//! guessing. The one thing that *is* fatal is a live emit that the gateway
//! rejects — at that point the run has data it cannot store, and silently
//! dropping it would leave a hole in a trend line nobody would notice.

use std::{path::PathBuf, sync::Arc};

use anyhow::{Context, Result};
use chrono::Utc;
use geo_core::{
    EmitMode, EmitOutcome, Engine, EngineConfig, EngineStatus, Family, GeoEmitter, GeoEvent,
    IngestEnvelope, InterrogationProbed, LlmClient, Prompt, PromptSet, ProbeOutcome, SCHEMA_VERSION,
    SovProbed,
    judge::{self, Judgement},
    scoring,
};
use tokio::sync::Semaphore;

use crate::{
    fixtures::FixtureBank,
    layer3::{InterrogationRow, Layer3, SovRow},
};

/// How many envelopes a dry run prints before summarising.
const DRY_RUN_SAMPLE: usize = 2;

/// Default repetitions per prompt per engine.
///
/// Three is the floor, not the target. Below three there is no distribution to
/// report and the intervals are wide enough to be useless; the runbook says to
/// raise it once the spend is understood.
pub const DEFAULT_REPETITIONS: u32 = 3;

/// Default in-flight probes per engine.
pub const DEFAULT_CONCURRENCY: u16 = 4;

/// Everything `geo probe` needs.
pub struct ProbeOptions {
    /// Which families to sweep.
    pub families: Vec<Family>,
    /// Which engines to probe.
    pub engines: Vec<Engine>,
    /// Samples per prompt per engine.
    pub repetitions: u32,
    /// Stop after this many prompts per family (a cheap smoke run).
    pub limit: Option<usize>,
    /// In-flight probes per engine.
    pub concurrency: usize,
    /// Write or print.
    pub mode: EmitMode,
    /// Where to write the Markdown report, if anywhere.
    pub markdown_out: Option<PathBuf>,
    /// Skip the judge entirely (3a-only sweeps, or a no-spend rehearsal).
    pub no_judge: bool,
}

/// Which transport answers a probe.
enum Responder {
    /// Real vendors.
    Live(LlmClient),
    /// Committed fixtures. Replaces the HTTPS call and nothing else.
    Fixture(FixtureBank),
}

impl Responder {
    async fn ask(
        &self,
        config: &EngineConfig,
        family: Family,
        prompt: &Prompt,
    ) -> ProbeOutcome {
        match self {
            Self::Live(client) => client.ask(config, &prompt.text).await,
            Self::Fixture(bank) => bank.answer(config.engine, family, &prompt.id),
        }
    }

    async fn judge(&self, config: &EngineConfig, engine: Engine, prompt: &Prompt, claim_id: &str, text: &str) -> ProbeOutcome {
        match self {
            Self::Live(client) => client.ask(config, text).await,
            Self::Fixture(bank) => ProbeOutcome::Answered(Box::new(geo_core::ProbeAnswer {
                text: bank.judge_reply(engine, &prompt.id, claim_id),
                cited_urls: Vec::new(),
                model: format!("fixture:{}", judge::JUDGE_ENGINE),
                input_tokens: None,
                output_tokens: None,
            })),
        }
    }

    fn is_fixture(&self) -> bool {
        matches!(self, Self::Fixture(_))
    }
}

/// One planned probe.
#[derive(Clone)]
struct Job {
    engine: Engine,
    prompt: Prompt,
    repetition: u32,
    run_id: String,
}

/// One completed probe.
struct Done {
    job: Job,
    outcome: ProbeOutcome,
}

/// Token and spend accounting for one sweep.
#[derive(Default)]
struct Spend {
    per_engine: std::collections::BTreeMap<String, (u64, u64, u64)>,
}

impl Spend {
    fn record(&mut self, engine: &str, input: Option<u64>, output: Option<u64>) {
        let entry = self.per_engine.entry(engine.to_string()).or_default();
        entry.0 += 1;
        entry.1 += input.unwrap_or(0);
        entry.2 += output.unwrap_or(0);
    }

    fn render(&self) -> String {
        let mut out = String::from("## Spend\n\n| engine | calls | input tokens | output tokens | list cost |\n|---|---|---|---|---|\n");
        let mut known_total = 0.0;
        let mut any_unpriced = false;
        for (engine, (calls, input, output)) in &self.per_engine {
            let pricing = Engine::parse(engine.trim_start_matches("judge:"))
                .map(Engine::pricing);
            let cost = pricing.and_then(|p| p.cost(*input, *output));
            if let Some(c) = cost {
                known_total += c;
            } else {
                any_unpriced = true;
            }
            out.push_str(&format!(
                "| {engine} | {calls} | {input} | {output} | {} |\n",
                cost.map_or_else(|| "unpriced".to_string(), |c| format!("${c:.4}"))
            ));
        }
        out.push_str(&format!("\nPriced total: **${known_total:.4}**"));
        if any_unpriced {
            out.push_str(
                ". Engines marked `unpriced` have no maintained rate card in this repository, so \
                 their tokens are reported and their cost is not guessed — see \
                 `geo_core::probe::Engine::pricing`.",
            );
        }
        out.push_str("\n\n");
        out
    }
}

/// Run a sweep.
pub async fn run(opts: ProbeOptions) -> Result<()> {
    let date = Utc::now().format("%Y-%m-%d").to_string();
    let responder = Arc::new(match opts.mode {
        EmitMode::DryRun => Responder::Fixture(FixtureBank::load()?),
        EmitMode::Live => Responder::Live(LlmClient::new()),
    });

    let mut layer = Layer3 {
        synthetic: responder.is_fixture(),
        provenance: format!(
            "`geo probe` sweep on {date} · {} repetition(s) per prompt per engine · {}",
            opts.repetitions,
            if responder.is_fixture() {
                "committed fixtures (no network, no keys)"
            } else {
                "live engines"
            }
        ),
        ..Default::default()
    };
    let mut spend = Spend::default();
    let mut events: Vec<GeoEvent> = Vec::new();

    println!("GEO probe — layers 3a/3b");
    println!("mode: {}", if responder.is_fixture() { "DRY RUN (fixtures, no network, no API key)" } else { "LIVE" });
    match geo_core::config::init_env() {
        geo_core::config::DotenvOutcome::Loaded { path } => println!("env: loaded {path} (real environment variables still win)"),
        geo_core::config::DotenvOutcome::NotFound => println!("env: no .env found (process environment only)"),
        geo_core::config::DotenvOutcome::Unreadable { reason } => {
            println!("env: .env present but unusable — {reason} (continuing on the process environment)");
        }
    }

    // Resolve engines once, up front, so a missing key is a headline and not a
    // surprise forty prompts in.
    let mut ready: Vec<EngineConfig> = Vec::new();
    for engine in &opts.engines {
        if responder.is_fixture() {
            ready.push(EngineConfig::new(*engine, format!("fixture:{engine}"), "not-used"));
            continue;
        }
        match EngineStatus::from_env(*engine) {
            EngineStatus::Ready(config) => ready.push(config),
            EngineStatus::MissingKey { engine, env } => {
                eprintln!(
                    "WARNING: skipping {engine} — {env} is not set. This is a SKIP, not zero \
                     share; every table below covers only the engines that ran."
                );
                layer
                    .skipped_engines
                    .push((engine.to_string(), env.to_string()));
            }
        }
    }
    if ready.is_empty() {
        anyhow::bail!(
            "no engine has a key set, so there is nothing to probe.\n\
             Set at least one of: {}.\n\
             Keys may live in a repository-root .env (real environment variables win over it).\n\
             Or run with --dry-run to exercise the harness against committed fixtures.",
            Engine::ALL
                .iter()
                .map(|e| e.key_env())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }

    println!();
    println!("-- engines --------------------------------------------------------");
    for config in &ready {
        println!("  {:<12} model {}", config.engine.as_str(), config.model);
    }

    for family in &opts.families {
        let set = family.load()?;
        let prompts: Vec<Prompt> = set
            .prompts
            .iter()
            .take(opts.limit.unwrap_or(usize::MAX))
            .cloned()
            .collect();

        println!();
        println!(
            "-- {family} set v{} · digest {} · {} prompt(s){} ------------",
            set.version,
            set.digest,
            prompts.len(),
            if opts.limit.is_some() {
                format!(" (limited from {})", set.len())
            } else {
                String::new()
            }
        );
        println!(
            "   run ids: {}",
            (1..=opts.repetitions)
                .map(|r| set.run_id(&date, r))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "   planned probes: {}",
            prompts.len() * ready.len() * opts.repetitions as usize
        );

        let done = sweep(&responder, &ready, &prompts, &set, &date, &opts).await;

        // Score, judge and build events.
        for Done { job, outcome } in done {
            let Some(answer) = outcome.answer() else {
                let ProbeOutcome::Failed { reason, .. } = &outcome else {
                    unreachable!("an outcome with no answer is a failure")
                };
                layer.failures.push((
                    job.engine.to_string(),
                    job.prompt.id.clone(),
                    reason.clone(),
                ));
                continue;
            };
            spend.record(job.engine.as_str(), answer.input_tokens, answer.output_tokens);

            match family {
                Family::Sov => {
                    let scored = scoring::score_sov(&answer.text);
                    layer.answers.push((job.engine.to_string(), answer.text.clone()));
                    layer.sov.push(SovRow {
                        engine: job.engine.to_string(),
                        prompt_id: job.prompt.id.clone(),
                        intent: job.prompt.intent.to_string(),
                        mentioned: scored.mentioned,
                        rank: scored.rank,
                        competitors: scored.competitors.clone(),
                        cited_urls: answer.cited_urls.clone(),
                        score: scored.score,
                        hedge: scored.hedge.map(str::to_string),
                    });
                    events.push(GeoEvent::Sov(SovProbed {
                        schema_version: SCHEMA_VERSION,
                        observed_at: Utc::now(),
                        run_id: job.run_id.clone(),
                        engine: job.engine.to_string(),
                        prompt_id: job.prompt.id.clone(),
                        prompt_text: job.prompt.text.clone(),
                        intent: job.prompt.intent.to_string(),
                        mentioned: scored.mentioned,
                        rank: scored.rank,
                        competitors: scored.competitors,
                        cited_urls: answer.cited_urls.clone(),
                        score: scored.score,
                    }));
                }
                Family::Interrogation => {
                    let judge_config = judge_config(&responder, &opts);
                    for claim in &job.prompt.claims {
                        let judgement = match &judge_config {
                            Some(config) => {
                                let prompt_text = judge::judge_prompt(
                                    &job.prompt,
                                    claim,
                                    job.engine,
                                    &answer.text,
                                );
                                let reply = responder
                                    .judge(config, job.engine, &job.prompt, &claim.id, &prompt_text)
                                    .await;
                                match reply.answer() {
                                    Some(a) => {
                                        spend.record(
                                            &format!("judge:{}", judge::JUDGE_ENGINE),
                                            a.input_tokens,
                                            a.output_tokens,
                                        );
                                        judge::parse_verdict(&a.text)
                                    }
                                    None => Judgement::unscored(
                                        "the judge call itself failed; this claim has no verdict \
                                         in this run",
                                    ),
                                }
                            }
                            None => Judgement::unscored(
                                "no judge was available (ANTHROPIC_API_KEY unset, or --no-judge)",
                            ),
                        };

                        let judge_model = judge::judge_model();
                        layer.interrogation.push(InterrogationRow {
                            engine: job.engine.to_string(),
                            prompt_id: job.prompt.id.clone(),
                            claim_id: claim.id.clone(),
                            verdict: judgement.verdict,
                            excerpt: judgement.excerpt.clone(),
                            reasoning: judgement.reasoning.clone(),
                            judge_model: judge_model.clone(),
                            cited_urls: answer.cited_urls.clone(),
                        });
                        events.push(GeoEvent::Interrogation(InterrogationProbed {
                            schema_version: SCHEMA_VERSION,
                            observed_at: Utc::now(),
                            run_id: job.run_id.clone(),
                            engine: job.engine.to_string(),
                            prompt_id: job.prompt.id.clone(),
                            prompt_text: job.prompt.text.clone(),
                            claim_id: claim.id.clone(),
                            verdict: judgement.verdict.to_string(),
                            reasoning: judgement.reasoning,
                            judge_model,
                            answer_excerpt: judgement.excerpt,
                            cited_urls: answer.cited_urls.clone(),
                            score: judgement.verdict.score(),
                        }));
                    }
                }
            }
        }
    }

    // The report, then the spend, then the write.
    println!();
    let report = layer.render();
    println!("{report}");
    let spend_block = spend.render();
    println!("{spend_block}");

    if let Some(path) = &opts.markdown_out {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        std::fs::write(path, format!("{report}\n{spend_block}"))
            .with_context(|| format!("could not write {}", path.display()))?;
        println!("wrote {}", path.display());
    }

    emit(&events, opts.mode, responder.is_fixture()).await
}

/// The judge's engine config, or `None` when there will be no judge.
fn judge_config(responder: &Responder, opts: &ProbeOptions) -> Option<EngineConfig> {
    if opts.no_judge {
        return None;
    }
    if responder.is_fixture() {
        return Some(EngineConfig::new(
            judge::JUDGE_ENGINE,
            format!("fixture:{}", judge::judge_model()),
            "not-used",
        ));
    }
    // The judge rides the Anthropic credential but usually a different model
    // from the `claude` probe engine, so retarget the resolved config.
    match EngineStatus::from_env(judge::JUDGE_ENGINE) {
        EngineStatus::Ready(config) => Some(config.with_model(judge::judge_model())),
        EngineStatus::MissingKey { .. } => None,
    }
}

/// Execute every planned probe, bounded per engine.
async fn sweep(
    responder: &Arc<Responder>,
    ready: &[EngineConfig],
    prompts: &[Prompt],
    set: &PromptSet,
    date: &str,
    opts: &ProbeOptions,
) -> Vec<Done> {
    let family = set.family;
    let mut tasks = tokio::task::JoinSet::new();

    for config in ready {
        // One permit pool per engine: providers rate-limit per account, so
        // saturating one must not slow the others.
        let permits = Arc::new(Semaphore::new(opts.concurrency.max(1)));
        for repetition in 1..=opts.repetitions {
            let run_id = set.run_id(date, repetition);
            for prompt in prompts {
                let job = Job {
                    engine: config.engine,
                    prompt: prompt.clone(),
                    repetition,
                    run_id: run_id.clone(),
                };
                let responder = Arc::clone(responder);
                let permits = Arc::clone(&permits);
                let config = config.clone();
                tasks.spawn(async move {
                    let _permit = permits.acquire().await;
                    let outcome = responder.ask(&config, family, &job.prompt).await;
                    Done { job, outcome }
                });
            }
        }
    }

    let mut done = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(item) => done.push(item),
            Err(e) => eprintln!("WARNING: a probe task panicked and its sample is lost: {e}"),
        }
    }
    // Deterministic order so two runs over the same fixtures print identically.
    done.sort_by(|a, b| {
        (a.job.engine.as_str(), &a.job.prompt.id, a.job.repetition).cmp(&(
            b.job.engine.as_str(),
            &b.job.prompt.id,
            b.job.repetition,
        ))
    });
    done
}

/// Write the events, or print what would be written.
async fn emit(events: &[GeoEvent], mode: EmitMode, synthetic: bool) -> Result<()> {
    if events.is_empty() {
        println!("no events to emit — every probe failed or the sweep was empty.");
        return Ok(());
    }

    match mode {
        EmitMode::DryRun => {
            println!(
                "-- first {} of {} envelope(s) that would be POSTed -----------------",
                DRY_RUN_SAMPLE.min(events.len()),
                events.len()
            );
            for event in events.iter().take(DRY_RUN_SAMPLE) {
                println!("{}", IngestEnvelope::build(event)?.to_pretty_json()?);
            }
            println!();
            println!(
                "DRY RUN — nothing written.{}",
                if synthetic {
                    " Every number above came from committed fixtures and is not a measurement."
                } else {
                    ""
                }
            );
            Ok(())
        }
        EmitMode::Live => {
            let emitter = GeoEmitter::from_env(mode)?;
            let outcomes = emitter.emit_all(events).await?;
            let ingested = outcomes
                .iter()
                .filter(|o| matches!(o, EmitOutcome::Ingested { .. }))
                .count();
            println!(
                "emitted {ingested}/{} layer-3 events to {}",
                events.len(),
                emitter.api_url()
            );
            println!(
                "re-running this sweep on the same day with the same prompt set is safe: the \
                 run_id carries the set digest and the repetition, so a replay appends a version \
                 to the same entity rather than inventing a second sample."
            );
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(mode: EmitMode) -> ProbeOptions {
        ProbeOptions {
            families: vec![Family::Sov],
            engines: Engine::ALL.to_vec(),
            repetitions: 1,
            limit: Some(2),
            concurrency: 2,
            mode,
            markdown_out: None,
            no_judge: false,
        }
    }

    #[tokio::test]
    async fn a_dry_run_needs_no_key_and_no_network() {
        // The property the whole CI story rests on.
        let result = run(options(EmitMode::DryRun)).await;
        assert!(result.is_ok(), "{result:?}");
    }

    #[tokio::test]
    async fn a_dry_run_scores_every_prompt_it_was_given() {
        let responder = Arc::new(Responder::Fixture(FixtureBank::load().expect("fixtures")));
        let set = Family::Sov.load().expect("set");
        let prompts: Vec<Prompt> = set.prompts.iter().take(3).cloned().collect();
        let ready: Vec<EngineConfig> = Engine::ALL
            .iter()
            .map(|e| EngineConfig::new(*e, "fixture", "x"))
            .collect();
        let done = sweep(
            &responder,
            &ready,
            &prompts,
            &set,
            "2026-08-11",
            &options(EmitMode::DryRun),
        )
        .await;
        assert_eq!(done.len(), 3 * Engine::ALL.len());
        assert!(done.iter().all(|d| d.outcome.answer().is_some()));
    }

    #[tokio::test]
    async fn repetitions_produce_distinct_run_ids_and_therefore_distinct_entities() {
        // The bug this guards: N samples collapsing into N versions of one
        // entity, so a reader folding by entity sees one sample where three
        // were taken.
        let responder = Arc::new(Responder::Fixture(FixtureBank::load().expect("fixtures")));
        let set = Family::Sov.load().expect("set");
        let prompts: Vec<Prompt> = set.prompts.iter().take(1).cloned().collect();
        let ready = vec![EngineConfig::new(Engine::Chatgpt, "fixture", "x")];
        let mut opts = options(EmitMode::DryRun);
        opts.repetitions = 3;
        let done = sweep(&responder, &ready, &prompts, &set, "2026-08-11", &opts).await;
        assert_eq!(done.len(), 3);

        let mut entity_ids: Vec<String> = done
            .iter()
            .map(|d| {
                GeoEvent::Sov(SovProbed {
                    schema_version: SCHEMA_VERSION,
                    observed_at: Utc::now(),
                    run_id: d.job.run_id.clone(),
                    engine: d.job.engine.to_string(),
                    prompt_id: d.job.prompt.id.clone(),
                    prompt_text: d.job.prompt.text.clone(),
                    intent: d.job.prompt.intent.to_string(),
                    mentioned: false,
                    rank: None,
                    competitors: vec![],
                    cited_urls: vec![],
                    score: 0.0,
                })
                .entity_id()
            })
            .collect();
        entity_ids.sort();
        entity_ids.dedup();
        assert_eq!(entity_ids.len(), 3, "repetitions collapsed into one entity");
    }

    #[test]
    fn the_spend_table_never_invents_a_price() {
        let mut spend = Spend::default();
        spend.record("claude", Some(1_000_000), Some(1_000_000));
        spend.record("chatgpt", Some(1_000_000), Some(1_000_000));
        let rendered = spend.render();
        assert!(rendered.contains("$30.0000"), "{rendered}");
        assert!(rendered.contains("unpriced"), "{rendered}");
    }

    #[test]
    fn a_fixture_responder_is_labelled_synthetic() {
        let responder = Responder::Fixture(FixtureBank::load().expect("fixtures"));
        assert!(responder.is_fixture());
        assert!(!Responder::Live(LlmClient::new()).is_fixture());
    }
}
