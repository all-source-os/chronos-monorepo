//! Layer 3 reporting — the tables, and the warnings that have to travel with
//! them.
//!
//! One renderer, one output. It emits Markdown, and the terminal prints that
//! Markdown verbatim: a baseline file that says something different from what
//! the operator saw on screen is a trap, and keeping two renderers in step is
//! exactly the kind of chore nobody does.
//!
//! Two producers feed the same aggregate:
//!
//! - `geo probe` fills it from a sweep it just ran (so it also has the answer
//!   texts, and can extract observed vocabulary).
//! - `geo report` fills it from `geo.sov.probed` / `geo.interrogation.probed`
//!   read back out of Core (so it spans weeks, but has no answer texts —
//!   payloads store the excerpt, not the whole answer).
//!
//! Anything the aggregate cannot compute from what it was given is stated as
//! missing rather than approximated.

use std::collections::{BTreeMap, BTreeSet};

use geo_core::{
    CompetitorSpec, Family, JudgeVerdict, RateEstimate, RateTally, SourceOwner,
    prompts::{Intent, PromptSet, Severity},
    scoring::{self, TermCount},
};

/// How many observed terms the vocabulary table prints.
const TOP_TERMS: usize = 40;

/// The framework's minimum observation window before a claimed relationship
/// between a change and a score is worth publishing.
pub const MIN_OBSERVATION_WEEKS: u32 = 12;

/// One scored share-of-voice probe.
#[derive(Debug, Clone)]
pub struct SovRow {
    /// Engine id.
    pub engine: String,
    /// Prompt id.
    pub prompt_id: String,
    /// Intent class as stored on the event.
    pub intent: String,
    /// Was AllSource named.
    pub mentioned: bool,
    /// Rank among named products.
    pub rank: Option<u32>,
    /// Competitor ids named.
    pub competitors: Vec<String>,
    /// URLs cited.
    pub cited_urls: Vec<String>,
    /// Reciprocal-rank score.
    pub score: f64,
    /// The hedge phrase, when the answer was a "don't know".
    pub hedge: Option<String>,
}

/// One graded factual claim.
#[derive(Debug, Clone)]
pub struct InterrogationRow {
    /// Engine id.
    pub engine: String,
    /// Prompt id.
    pub prompt_id: String,
    /// Claim id.
    pub claim_id: String,
    /// The verdict.
    pub verdict: JudgeVerdict,
    /// The quoted answer text the verdict rests on.
    pub excerpt: String,
    /// The judge's argument.
    pub reasoning: String,
    /// Which model judged.
    pub judge_model: String,
    /// URLs the engine cited in the answer.
    pub cited_urls: Vec<String>,
}

/// Everything layer 3 reporting needs.
#[derive(Debug, Default)]
pub struct Layer3 {
    /// Scored SOV probes.
    pub sov: Vec<SovRow>,
    /// Graded claims.
    pub interrogation: Vec<InterrogationRow>,
    /// `(engine, answer text)` for vocabulary extraction. Only a fresh sweep
    /// has these; the stored payloads keep an excerpt, not the whole answer.
    pub answers: Vec<(String, String)>,
    /// Which engines were skipped for want of a key, and which variable to set.
    pub skipped_engines: Vec<(String, String)>,
    /// Probes that failed after retries: `(engine, prompt_id, reason)`.
    pub failures: Vec<(String, String, String)>,
    /// A line describing what produced this aggregate.
    pub provenance: String,
    /// Set to true when the data came from committed fixtures.
    pub synthetic: bool,
}

impl Layer3 {
    /// Whether there is anything at all to report.
    pub fn is_empty(&self) -> bool {
        self.sov.is_empty() && self.interrogation.is_empty()
    }

    /// Engines seen, in contract order where known.
    fn engines(&self) -> Vec<String> {
        let mut set: BTreeSet<String> = BTreeSet::new();
        for row in &self.sov {
            set.insert(row.engine.clone());
        }
        for row in &self.interrogation {
            set.insert(row.engine.clone());
        }
        set.into_iter().collect()
    }

    /// Render the whole layer-3 section as Markdown.
    pub fn render(&self) -> String {
        let mut out = String::new();
        if self.synthetic {
            out.push_str(
                "> **SYNTHETIC RUN — NOT A MEASUREMENT.** Every number below was computed \
                 from the committed fixture replies in\n> \
                 `tooling/geo/geo-cli/tests/fixtures/probe/`, not from a live engine. It \
                 proves the harness works; it says\n> nothing whatsoever about how the \
                 engines actually answer. Do not copy any of it into a baseline document.\n\n",
            );
        }
        if !self.provenance.is_empty() {
            out.push_str(&format!("_{}_\n\n", self.provenance));
        }
        self.render_skips(&mut out);
        self.render_sov(&mut out);
        self.render_interrogation(&mut out);
        self.render_sources(&mut out);
        self.render_vocabulary(&mut out);
        self.render_backlog(&mut out);
        out
    }

    fn render_skips(&self, out: &mut String) {
        if !self.skipped_engines.is_empty() {
            out.push_str("## Engines skipped\n\n");
            out.push_str(
                "These engines were **not probed**. Their absence is not zero share — read \
                 every table below as covering only the engines listed in it.\n\n",
            );
            for (engine, env) in &self.skipped_engines {
                out.push_str(&format!("- `{engine}` — no key. Set `{env}`.\n"));
            }
            out.push('\n');
        }
        if !self.failures.is_empty() {
            out.push_str(&format!(
                "## Failed probes ({})\n\nRecorded rather than discarded — a partial sweep is \
                 worth more than a lost one, but these prompts have no sample in this run.\n\n",
                self.failures.len()
            ));
            for (engine, prompt_id, reason) in self.failures.iter().take(20) {
                out.push_str(&format!("- `{engine}` / `{prompt_id}` — {reason}\n"));
            }
            if self.failures.len() > 20 {
                out.push_str(&format!("- … and {} more\n", self.failures.len() - 20));
            }
            out.push('\n');
        }
    }

    // ── 3a ────────────────────────────────────────────────────────────────

    fn render_sov(&self, out: &mut String) {
        out.push_str("## Layer 3a — share of voice\n\n");
        if self.sov.is_empty() {
            out.push_str(
                "No `geo.sov.probed` rows. Run `geo probe --family sov` (see the runbook).\n\n",
            );
            return;
        }

        out.push_str(
            "> **Share of voice alone is a vanity metric.** It tells you whether you are \
             appearing in answers,\n> not whether anyone is buying anything. It is a trend \
             instrument and an input to layer 5 — never the KPI.\n> Read it next to layer 1 \
             (arrivals) and layer 4 (self-report), and never on its own.\n\n",
        );
        out.push_str(&format!(
            "Every rate is a **95% Wilson score interval**, not a point estimate. Two cells \
             whose intervals overlap have not\nmoved relative to each other, however different \
             their percentages look. Do not publish a claimed relationship\nbetween a change \
             and a score over a window shorter than **{MIN_OBSERVATION_WEEKS} weeks**.\n\n"
        ));

        let engines = self.engines();

        // Mention rate, per intent class, per engine. Never blended across
        // classes: being named on "alternatives to Mem0" and on "my agent
        // forgets" are different wins.
        out.push_str("### Mention rate by intent class\n\n");
        out.push_str("| intent | ");
        out.push_str(&engines.join(" | "));
        out.push_str(" | all engines |\n|");
        for _ in 0..engines.len() + 2 {
            out.push_str("---|");
        }
        out.push('\n');

        let mut intents: Vec<String> = self
            .sov
            .iter()
            .map(|r| r.intent.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        intents.sort_by_key(|i| {
            Intent::SOV
                .iter()
                .position(|s| s.as_str() == i)
                .unwrap_or(usize::MAX)
        });

        for intent in &intents {
            out.push_str(&format!("| {intent} "));
            for engine in &engines {
                let tally = self.mention_tally(|r| &r.engine == engine && &r.intent == intent);
                out.push_str(&format!("| {} ", cell(tally)));
            }
            let all = self.mention_tally(|r| &r.intent == intent);
            out.push_str(&format!("| {} |\n", cell(all)));
        }
        out.push_str("| **all classes** ");
        for engine in &engines {
            let tally = self.mention_tally(|r| &r.engine == engine);
            out.push_str(&format!("| {} ", cell(tally)));
        }
        out.push_str(&format!("| {} |\n\n", cell(self.mention_tally(|_| true))));

        // Mean reciprocal rank: presence and position, separated from the
        // headline on purpose.
        out.push_str("### Mean reciprocal rank (position when named)\n\n");
        out.push_str("| engine | probes | mentions | mean 1/rank | best rank |\n|---|---|---|---|---|\n");
        for engine in &engines {
            let rows: Vec<&SovRow> = self.sov.iter().filter(|r| &r.engine == engine).collect();
            let mentions = rows.iter().filter(|r| r.mentioned).count();
            let mrr = if rows.is_empty() {
                0.0
            } else {
                rows.iter().map(|r| r.score).sum::<f64>() / rows.len() as f64
            };
            let best = rows.iter().filter_map(|r| r.rank).min();
            out.push_str(&format!(
                "| {engine} | {} | {mentions} | {mrr:.3} | {} |\n",
                rows.len(),
                best.map_or("—".to_string(), |r| r.to_string()),
            ));
        }
        out.push('\n');

        // Competitor share, so SOV is relative rather than absolute.
        out.push_str("### Competitor share (same answers, same denominator)\n\n");
        out.push_str("| product | kind | all engines | ");
        out.push_str(&engines.join(" | "));
        out.push_str(" |\n|---|---|---|");
        for _ in &engines {
            out.push_str("---|");
        }
        out.push('\n');

        let total_answers = self.sov.len() as u64;
        let mut rows: Vec<(String, &'static str, u64)> = Vec::new();
        rows.push((
            "**AllSource (us)**".to_string(),
            "us",
            self.sov.iter().filter(|r| r.mentioned).count() as u64,
        ));
        for spec in geo_core::COMPETITORS {
            let count = self
                .sov
                .iter()
                .filter(|r| r.competitors.iter().any(|c| c == spec.id))
                .count() as u64;
            if count > 0 {
                rows.push((spec.label.to_string(), spec.kind.as_str(), count));
            }
        }
        rows.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
        for (label, kind, count) in rows {
            out.push_str(&format!(
                "| {label} | {kind} | {} ",
                RateEstimate::new(count, total_answers).render()
            ));
            for engine in &engines {
                let engine_rows: Vec<&SovRow> =
                    self.sov.iter().filter(|r| &r.engine == engine).collect();
                let named = engine_rows
                    .iter()
                    .filter(|r| {
                        if label.starts_with("**AllSource") {
                            r.mentioned
                        } else {
                            competitor_id_for_label(&label)
                                .is_some_and(|id| r.competitors.iter().any(|c| c == id))
                        }
                    })
                    .count() as u64;
                out.push_str(&format!(
                    "| {} ",
                    RateEstimate::new(named, engine_rows.len() as u64).render()
                ));
            }
            out.push_str("|\n");
        }
        out.push('\n');

        // Knowledge gaps: the cheapest share to win.
        let gaps: Vec<&SovRow> = self.sov.iter().filter(|r| r.hedge.is_some()).collect();
        out.push_str(&format!("### Knowledge gaps ({} hedged answers)\n\n", gaps.len()));
        if gaps.is_empty() {
            out.push_str("No engine hedged on any prompt in this run.\n\n");
        } else {
            out.push_str(
                "Prompts where the engine said it did not know. These are content-production \
                 targets, not losses:\nnobody else owns the answer either.\n\n\
                 | engine | prompt | hedge |\n|---|---|---|\n",
            );
            let mut seen: BTreeSet<(String, String)> = BTreeSet::new();
            for row in gaps {
                if seen.insert((row.engine.clone(), row.prompt_id.clone())) {
                    out.push_str(&format!(
                        "| {} | `{}` | {} |\n",
                        row.engine,
                        row.prompt_id,
                        row.hedge.as_deref().unwrap_or("")
                    ));
                }
            }
            out.push('\n');
        }
    }

    fn mention_tally(&self, filter: impl Fn(&SovRow) -> bool) -> RateTally {
        let mut tally = RateTally::default();
        for row in self.sov.iter().filter(|r| filter(r)) {
            tally.record(row.mentioned);
        }
        tally
    }

    // ── 3b ────────────────────────────────────────────────────────────────

    fn render_interrogation(&self, out: &mut String) {
        out.push_str("## Layer 3b — interrogation accuracy\n\n");
        if self.interrogation.is_empty() {
            out.push_str(
                "No `geo.interrogation.probed` rows. Run `geo probe --family interrogation`.\n\n",
            );
            return;
        }

        out.push_str(
            "`absent` is kept out of the accuracy rate's numerator **and** reported \
             separately: a model that says nothing\nabout our licence has not got it wrong, and \
             the fix is different (write the page, versus correct the page).\n`unscored` means \
             the judge's own reply could not be read; those rows are excluded from the \
             denominator entirely.\n\n",
        );

        let engines = self.engines();
        out.push_str("### Verdicts by engine\n\n");
        out.push_str("| engine | accurate | partially | inaccurate | absent | unscored | accuracy (accurate/scored) |\n|---|---|---|---|---|---|---|\n");
        for engine in engines.iter().chain(std::iter::once(&"ALL".to_string())) {
            let rows: Vec<&InterrogationRow> = self
                .interrogation
                .iter()
                .filter(|r| engine == "ALL" || &r.engine == engine)
                .collect();
            let count = |v: JudgeVerdict| rows.iter().filter(|r| r.verdict == v).count();
            let scored = rows.iter().filter(|r| r.verdict.is_scored()).count() as u64;
            let accurate = count(JudgeVerdict::Accurate) as u64;
            out.push_str(&format!(
                "| {} | {} | {} | {} | {} | {} | {} |\n",
                if engine == "ALL" { "**all**" } else { engine },
                accurate,
                count(JudgeVerdict::PartiallyAccurate),
                count(JudgeVerdict::Inaccurate),
                count(JudgeVerdict::Absent),
                count(JudgeVerdict::Unscored),
                RateEstimate::new(accurate, scored).render(),
            ));
        }
        out.push('\n');

        // Every failing claim, quoted verbatim, with the judge's argument.
        let failing: Vec<&InterrogationRow> = self
            .interrogation
            .iter()
            .filter(|r| r.verdict.needs_remediation())
            .collect();
        // One entry per distinct (engine, claim, excerpt): N repetitions of the
        // same wrong answer are one finding seen N times, not N findings.
        let mut distinct: Vec<(&InterrogationRow, usize)> = Vec::new();
        for row in &failing {
            match distinct.iter_mut().find(|(seen, _)| {
                seen.engine == row.engine
                    && seen.claim_id == row.claim_id
                    && seen.excerpt == row.excerpt
            }) {
                Some((_, count)) => *count += 1,
                None => distinct.push((row, 1)),
            }
        }
        out.push_str(&format!(
            "### Wrong claims, quoted verbatim ({} distinct, {} answer(s))\n\n",
            distinct.len(),
            failing.len()
        ));
        if failing.is_empty() {
            out.push_str(
                "**No engine got any claim wrong in this run.** That is a real result and is \
                 stated as one — nothing has been\ninvented to fill this section.\n\n",
            );
        } else {
            let set = claim_set();
            for (row, occurrences) in &distinct {
                let severity = severity_of(&set, &row.claim_id);
                out.push_str(&format!(
                    "**`{}`** · engine `{}` · claim `{}` · severity **{}** · verdict **{}** · seen in {} answer(s)\n\n",
                    row.prompt_id,
                    row.engine,
                    row.claim_id,
                    severity.map_or("?".to_string(), |s| s.to_string()),
                    row.verdict,
                    occurrences
                ));
                out.push_str(&format!("> {}\n\n", quote(&row.excerpt)));
                out.push_str(&format!(
                    "Judge (`{}`): {}\n\n",
                    row.judge_model,
                    row.reasoning.trim()
                ));
                if let Some(source) = source_of(&set, &row.claim_id) {
                    out.push_str(&format!("Ground truth lives in `{source}`.\n\n"));
                }
                if !row.cited_urls.is_empty() {
                    out.push_str(&format!(
                        "Cited in that answer: {}\n\n",
                        row.cited_urls
                            .iter()
                            .map(|u| format!("`{u}`"))
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                out.push_str("---\n\n");
            }
        }

        let unscored: Vec<&InterrogationRow> = self
            .interrogation
            .iter()
            .filter(|r| r.verdict == JudgeVerdict::Unscored)
            .collect();
        if !unscored.is_empty() {
            out.push_str(&format!(
                "### Unscored ({}) — the judge's reply could not be read\n\n",
                unscored.len()
            ));
            for row in unscored.iter().take(10) {
                out.push_str(&format!(
                    "- `{}` / `{}` / `{}` — {}\n",
                    row.engine,
                    row.prompt_id,
                    row.claim_id,
                    row.reasoning.trim()
                ));
            }
            out.push('\n');
        }
    }

    // ── sources ───────────────────────────────────────────────────────────

    fn render_sources(&self, out: &mut String) {
        // Keyed by (host, owner), not host alone: `github.com` holds both our
        // own repository and everyone else's, and collapsing them would launder
        // a third-party citation into "ours".
        let mut hosts: BTreeMap<(String, SourceOwner), u64> = BTreeMap::new();
        let all_urls = self
            .sov
            .iter()
            .flat_map(|r| r.cited_urls.iter())
            .chain(self.interrogation.iter().flat_map(|r| r.cited_urls.iter()));
        for url in all_urls {
            let host = scoring::host_of(url);
            let owner = scoring::classify_source(url);
            *hosts.entry((host, owner)).or_insert(0) += 1;
        }

        out.push_str("## Source attribution\n\n");
        if hosts.is_empty() {
            out.push_str(
                "No engine in this run exposed a citation, and no answer wrote a URL into its \
                 prose. Two of the four\nengines return citations natively; the other two only \
                 ever contribute URLs they happened to type.\n\n",
            );
            return;
        }
        out.push_str(
            "A **third-party page carrying a wrong claim is the highest-value remediation \
             target in the whole programme**:\nwe cannot edit it, so it has to be corrected at \
             source or out-published.\n\n| host | owner | citations |\n|---|---|---|\n",
        );
        let mut rows: Vec<(&(String, SourceOwner), &u64)> = hosts.iter().collect();
        rows.sort_by(|a, b| b.1.cmp(a.1).then_with(|| a.0.cmp(b.0)));
        for ((host, owner), count) in rows {
            out.push_str(&format!(
                "| `{host}` | {} | {count} |\n",
                match owner {
                    SourceOwner::Ours => "ours",
                    SourceOwner::ThirdParty => "**third-party**",
                }
            ));
        }
        out.push('\n');
    }

    // ── vocabulary ────────────────────────────────────────────────────────

    fn render_vocabulary(&self, out: &mut String) {
        out.push_str("## Observed vocabulary\n\n");
        if self.answers.is_empty() {
            out.push_str(
                "Not available from stored events — the payloads keep the excerpt a verdict \
                 rests on, not the whole answer.\nRun `geo probe` to extract vocabulary from a \
                 fresh sweep.\n\n",
            );
            return;
        }
        let terms: Vec<TermCount> = scoring::key_terms(&self.answers);
        out.push_str(&format!(
            "The words the engines actually use for this category, ranked by how many answers \
             contained them.\nThis is **not** filtered against our own copy: the overlap, or \
             the lack of it, is the finding. {} terms over {} answers.\n\n\
             | term | answers | engines | is a product name |\n|---|---|---|---|\n",
            terms.len(),
            self.answers.len()
        ));
        for term in terms.iter().take(TOP_TERMS) {
            out.push_str(&format!(
                "| {} | {} | {} | {} |\n",
                term.term,
                term.documents,
                term.engines.iter().cloned().collect::<Vec<_>>().join(", "),
                if term.is_product { "yes" } else { "" }
            ));
        }
        if terms.len() > TOP_TERMS {
            out.push_str(&format!(
                "\n_{} further terms met the threshold and are omitted here._\n",
                terms.len() - TOP_TERMS
            ));
        }
        out.push('\n');
    }

    // ── backlog ───────────────────────────────────────────────────────────

    fn render_backlog(&self, out: &mut String) {
        out.push_str("## Remediation backlog (frequency × severity)\n\n");
        let failing: Vec<&InterrogationRow> = self
            .interrogation
            .iter()
            .filter(|r| r.verdict.needs_remediation())
            .collect();
        if failing.is_empty() {
            out.push_str("Nothing to remediate from this run.\n\n");
            return;
        }
        let set = claim_set();
        let mut by_claim: BTreeMap<String, ClaimTally> = BTreeMap::new();
        for row in &failing {
            let entry = by_claim.entry(row.claim_id.clone()).or_default();
            entry.wrong_answers += 1;
            entry.engines.insert(row.engine.clone());
            for url in &row.cited_urls {
                if scoring::classify_source(url) == SourceOwner::ThirdParty {
                    entry.third_party_hosts.insert(scoring::host_of(url));
                }
            }
        }
        let mut ranked: Vec<(String, ClaimTally)> = by_claim.into_iter().collect();
        for (claim, tally) in &mut ranked {
            tally.score =
                tally.wrong_answers as f64 * severity_of(&set, claim).map_or(1.0, Severity::weight);
        }
        ranked.sort_by(|a, b| {
            b.1.score
                .partial_cmp(&a.1.score)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.0.cmp(&b.0))
        });

        out.push_str(
            "| # | claim | severity | wrong in | engines | score | fix (edit this) | likely third-party source |\n\
             |---|---|---|---|---|---|---|---|\n",
        );
        for (i, (claim, tally)) in ranked.iter().enumerate() {
            out.push_str(&format!(
                "| {} | `{claim}` | {} | {} answer(s) | {} | {:.0} | `{}` | {} |\n",
                i + 1,
                severity_of(&set, claim).map_or("?".to_string(), |s| s.to_string()),
                tally.wrong_answers,
                tally.engines.iter().cloned().collect::<Vec<_>>().join(", "),
                tally.score,
                source_of(&set, claim).unwrap_or("—"),
                if tally.third_party_hosts.is_empty() {
                    "—".to_string()
                } else {
                    tally
                        .third_party_hosts
                        .iter()
                        .map(|h| format!("`{h}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                }
            ));
        }
        out.push_str(
            "\nScore is `frequency × severity weight` (critical 8, high 4, medium 2, low 1). \
             The `fix` column is the file in this\nrepository that defines the ground truth the \
             engine contradicted — that is where a correction starts.\n\n",
        );
    }
}

/// One claim's backlog row while it is being accumulated.
#[derive(Default)]
struct ClaimTally {
    /// Answers that got this claim wrong.
    wrong_answers: u64,
    /// Which engines got it wrong.
    engines: BTreeSet<String>,
    /// Third-party hosts cited in those answers — the likely upstream source
    /// of the wrong claim, and the only lead we get on a page we cannot edit.
    third_party_hosts: BTreeSet<String>,
    /// `wrong_answers × severity weight`.
    score: f64,
}

/// Render one rate cell.
fn cell(tally: RateTally) -> String {
    tally.estimate().render()
}

/// Flatten a quoted excerpt onto one blockquote line.
fn quote(text: &str) -> String {
    text.replace('\n', " ").trim().to_string()
}

/// The compiled-in interrogation set, for claim severity and source lookup.
///
/// The set is validated by a test in `geo-core`, so a build that passed CI
/// cannot fail this — but reporting must never panic on a data path, so an
/// unreadable set degrades to "no severity known" rather than aborting a run
/// that already cost money.
fn claim_set() -> PromptSet {
    Family::Interrogation.load().unwrap_or(PromptSet {
        family: Family::Interrogation,
        version: 0,
        prompts: Vec::new(),
        digest: String::new(),
    })
}

fn severity_of(set: &PromptSet, claim_id: &str) -> Option<Severity> {
    set.prompts
        .iter()
        .flat_map(|p| p.claims.iter())
        .find(|c| c.id == claim_id)
        .map(|c| c.severity)
}

fn source_of<'a>(set: &'a PromptSet, claim_id: &str) -> Option<&'a str> {
    set.prompts
        .iter()
        .flat_map(|p| p.claims.iter())
        .find(|c| c.id == claim_id)
        .map(|c| c.source.as_str())
}

/// Look a competitor id up from the label used in the share table.
fn competitor_id_for_label(label: &str) -> Option<&'static str> {
    geo_core::COMPETITORS
        .iter()
        .find(|c: &&CompetitorSpec| c.label == label)
        .map(|c| c.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sov_row(engine: &str, intent: &str, mentioned: bool, rank: Option<u32>) -> SovRow {
        SovRow {
            engine: engine.to_string(),
            prompt_id: "p1".to_string(),
            intent: intent.to_string(),
            mentioned,
            rank,
            competitors: vec!["mem0".to_string()],
            cited_urls: vec![],
            score: rank.map_or(0.0, |r| 1.0 / f64::from(r)),
            hedge: None,
        }
    }

    fn wrong_row(claim: &str) -> InterrogationRow {
        InterrogationRow {
            engine: "chatgpt".to_string(),
            prompt_id: "int-pricing".to_string(),
            claim_id: claim.to_string(),
            verdict: JudgeVerdict::Inaccurate,
            excerpt: "AllSource offers a free tier".to_string(),
            reasoning: "There is no free hosted plan.".to_string(),
            judge_model: "claude-opus-5".to_string(),
            cited_urls: vec!["https://blog.example.dev/x".to_string()],
        }
    }

    #[test]
    fn an_empty_aggregate_says_so_rather_than_printing_zeros() {
        let rendered = Layer3::default().render();
        assert!(rendered.contains("No `geo.sov.probed` rows"), "{rendered}");
        assert!(
            rendered.contains("No `geo.interrogation.probed` rows"),
            "{rendered}"
        );
    }

    #[test]
    fn the_vanity_metric_warning_travels_with_the_numbers() {
        // The framework is blunt about this and so is the report: the warning
        // is not a doc that can be skipped, it is in the output itself.
        let mut layer = Layer3::default();
        layer.sov.push(sov_row("chatgpt", "category", true, Some(1)));
        let rendered = layer.render();
        assert!(rendered.contains("vanity metric"), "{rendered}");
    }

    #[test]
    fn every_rate_is_printed_with_an_interval() {
        let mut layer = Layer3::default();
        for _ in 0..3 {
            layer.sov.push(sov_row("chatgpt", "category", true, Some(2)));
            layer.sov.push(sov_row("chatgpt", "category", false, None));
        }
        let rendered = layer.render();
        // 3/6 = 50%, and it must never appear without its bracketed interval.
        assert!(rendered.contains("n=6"), "{rendered}");
        assert!(rendered.contains('['), "{rendered}");
    }

    #[test]
    fn intent_classes_are_reported_separately() {
        let mut layer = Layer3::default();
        layer.sov.push(sov_row("chatgpt", "category", true, Some(1)));
        layer.sov.push(sov_row("chatgpt", "comparison", false, None));
        let rendered = layer.render();
        assert!(rendered.contains("| category "), "{rendered}");
        assert!(rendered.contains("| comparison "), "{rendered}");
    }

    #[test]
    fn a_synthetic_run_is_labelled_before_anything_else() {
        let layer = Layer3 {
            synthetic: true,
            ..Default::default()
        };
        let rendered = layer.render();
        assert!(rendered.starts_with("> **SYNTHETIC RUN"), "{rendered}");
    }

    #[test]
    fn a_clean_interrogation_run_says_so_instead_of_inventing_a_finding() {
        let mut layer = Layer3::default();
        layer.interrogation.push(InterrogationRow {
            verdict: JudgeVerdict::Accurate,
            ..wrong_row("pricing.tiers")
        });
        let rendered = layer.render();
        assert!(
            rendered.contains("No engine got any claim wrong"),
            "{rendered}"
        );
        assert!(rendered.contains("Nothing to remediate"), "{rendered}");
    }

    #[test]
    fn a_wrong_claim_is_quoted_verbatim_with_the_judges_reasoning() {
        let mut layer = Layer3::default();
        layer.interrogation.push(wrong_row("pricing.no-free-plan"));
        let rendered = layer.render();
        assert!(
            rendered.contains("> AllSource offers a free tier"),
            "excerpt not quoted: {rendered}"
        );
        assert!(
            rendered.contains("There is no free hosted plan."),
            "judge reasoning missing: {rendered}"
        );
        assert!(rendered.contains("claude-opus-5"), "judge model missing");
    }

    #[test]
    fn the_backlog_orders_by_frequency_times_severity() {
        let mut layer = Layer3::default();
        // One critical miss (weight 8) versus two medium ones (weight 2 each):
        // 8 beats 4, so the critical claim must come first.
        layer.interrogation.push(wrong_row("pricing.no-free-plan"));
        for _ in 0..2 {
            layer.interrogation.push(InterrogationRow {
                claim_id: "perf.latency".to_string(),
                ..wrong_row("perf.latency")
            });
        }
        let rendered = layer.render();
        let critical = rendered.find("pricing.no-free-plan").expect("critical row");
        let medium = rendered.find("| `perf.latency`").expect("medium row");
        assert!(critical < medium, "backlog is not severity-ordered");
    }

    #[test]
    fn the_backlog_names_the_repository_file_that_defines_the_ground_truth() {
        let mut layer = Layer3::default();
        layer.interrogation.push(wrong_row("durability.wal-parquet"));
        let rendered = layer.render();
        assert!(rendered.contains("CLAUDE.md"), "{rendered}");
    }

    #[test]
    fn third_party_hosts_are_separated_from_ours() {
        let mut layer = Layer3::default();
        layer.sov.push(SovRow {
            cited_urls: vec![
                "https://www.all-source.xyz/pricing".to_string(),
                "https://blog.example.dev/post".to_string(),
            ],
            ..sov_row("perplexity", "category", true, Some(1))
        });
        let rendered = layer.render();
        assert!(rendered.contains("| `www.all-source.xyz` | ours |"), "{rendered}");
        assert!(
            rendered.contains("| `blog.example.dev` | **third-party** |"),
            "{rendered}"
        );
    }

    #[test]
    fn a_skipped_engine_is_reported_as_a_skip_not_as_zero_share() {
        let mut layer = Layer3::default();
        layer
            .skipped_engines
            .push(("gemini".to_string(), "GEMINI_API_KEY".to_string()));
        let rendered = layer.render();
        assert!(rendered.contains("Engines skipped"), "{rendered}");
        assert!(rendered.contains("GEMINI_API_KEY"), "{rendered}");
        assert!(rendered.contains("not zero share"), "{rendered}");
    }

    #[test]
    fn vocabulary_is_absent_rather_than_approximated_when_texts_are_gone() {
        let mut layer = Layer3::default();
        layer.sov.push(sov_row("chatgpt", "category", true, Some(1)));
        let rendered = layer.render();
        assert!(
            rendered.contains("Not available from stored events"),
            "{rendered}"
        );
    }
}
