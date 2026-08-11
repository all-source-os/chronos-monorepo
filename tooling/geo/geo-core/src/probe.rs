//! The engines, and how one prompt becomes one answer.
//!
//! Four vendors, four JSON shapes, one [`ProbeAnswer`]. Everything downstream —
//! scoring, judging, reporting — sees only the normalised answer, so adding a
//! fifth engine never touches the scorer.
//!
//! ## Three rules this module exists to enforce
//!
//! 1. **A missing key is a loud skip, never a zero.** A silently skipped engine
//!    reads downstream as "we lost all our share on Gemini", and would send the
//!    optimization loop in prompt 027 chasing a phantom. [`EngineStatus`] makes
//!    the skip a value the caller must handle and print.
//! 2. **A partial run keeps what it got.** Per-probe failures come back as
//!    [`ProbeOutcome::Failed`] and are counted, not thrown. Losing 40 good
//!    answers because the 41st rate-limited is the worst possible trade when
//!    each answer costs money.
//! 3. **`--dry-run` exercises the real code.** The fixture responder swaps out
//!    the HTTP call and nothing else: parsing, scoring, judging and emission
//!    are the same code paths a live run takes. That is what makes the CI
//!    tests evidence rather than decoration.
//!
//! ## Model ids are configuration, not constants
//!
//! Every engine's model id is overridable by environment variable, and the
//! resolved id is printed before the run and stored on every answer. Vendor
//! model names churn faster than this repository does; a hard-coded id that
//! 404s mid-sweep would burn the run, and one that silently resolves to a
//! different generation would corrupt a 12-week trend.

use std::time::Duration;

use serde_json::{Value, json};

/// Output-token ceiling per probe.
///
/// Generous on purpose: a truncated answer scores as "did not mention us",
/// which is a false negative on the metric the whole programme optimises. On
/// engines that think before answering, this budget covers both.
pub const MAX_OUTPUT_TOKENS: u32 = 4000;

/// Per-request timeout. Reasoning models are slow; a 429 retry is cheaper than
/// a false failure.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

/// Attempts per probe, including the first.
const MAX_ATTEMPTS: u32 = 4;

/// First backoff step; doubles per attempt.
const BACKOFF_BASE: Duration = Duration::from_secs(2);

/// Longest we will ever honour a `Retry-After`.
const BACKOFF_CAP: Duration = Duration::from_secs(60);

/// One generative engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Engine {
    /// OpenAI — `payload.engine == "chatgpt"`, per the event contract.
    Chatgpt,
    /// Anthropic.
    Claude,
    /// Perplexity.
    Perplexity,
    /// Google Gemini.
    Gemini,
}

impl Engine {
    /// Every engine, in report order.
    pub const ALL: [Self; 4] = [Self::Chatgpt, Self::Claude, Self::Perplexity, Self::Gemini];

    /// The `payload.engine` string fixed by the event contract.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Chatgpt => "chatgpt",
            Self::Claude => "claude",
            Self::Perplexity => "perplexity",
            Self::Gemini => "gemini",
        }
    }

    /// Parse a CLI/wire string.
    pub fn parse(s: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|e| e.as_str() == s)
    }

    /// Environment variable holding this engine's API key.
    pub fn key_env(self) -> &'static str {
        match self {
            Self::Chatgpt => "OPENAI_API_KEY",
            Self::Claude => "ANTHROPIC_API_KEY",
            Self::Perplexity => "PERPLEXITY_API_KEY",
            Self::Gemini => "GEMINI_API_KEY",
        }
    }

    /// Environment variable overriding this engine's model id.
    pub fn model_env(self) -> &'static str {
        match self {
            Self::Chatgpt => "GEO_CHATGPT_MODEL",
            Self::Claude => "GEO_CLAUDE_MODEL",
            Self::Perplexity => "GEO_PERPLEXITY_MODEL",
            Self::Gemini => "GEO_GEMINI_MODEL",
        }
    }

    /// Model id used when the override is unset.
    ///
    /// **Verify these against the vendor's current model list before the first
    /// live sweep** — see the layer-3 section of the runbook. They are a
    /// starting point, not a guarantee.
    pub fn default_model(self) -> &'static str {
        match self {
            Self::Chatgpt => "gpt-5",
            // Per the `claude-api` skill's current model table. Override to
            // `claude-haiku-4-5` for a cheap smoke sweep.
            Self::Claude => "claude-opus-5",
            Self::Perplexity => "sonar-pro",
            Self::Gemini => "gemini-2.5-pro",
        }
    }

    /// Whether the vendor returns citations of its own. Where it does not, the
    /// only URLs we can record are the ones the model wrote into its prose —
    /// which is a weaker signal and is labelled as such in the report.
    pub fn has_native_citations(self) -> bool {
        matches!(self, Self::Perplexity | Self::Gemini)
    }

    /// List price per million tokens, and when it was last checked.
    ///
    /// Only Anthropic's is filled in, because it is the only one this
    /// repository has a maintained source for (the `claude-api` skill). The
    /// rest report token counts and an explicit "unpriced" — a made-up price
    /// on a spend report is worse than no price.
    pub fn pricing(self) -> Pricing {
        match self {
            Self::Claude => Pricing {
                input_per_mtok: Some(5.00),
                output_per_mtok: Some(25.00),
                currency: "USD",
                model: "claude-opus-5",
                as_of: "2026-06-24",
                source: "claude-api skill, Current Models table",
            },
            _ => Pricing {
                input_per_mtok: None,
                output_per_mtok: None,
                currency: "USD",
                model: "",
                as_of: "",
                source: "unpriced — no maintained source in this repository",
            },
        }
    }
}

impl std::fmt::Display for Engine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// List pricing for one engine, with its provenance attached.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pricing {
    /// USD per million input tokens, when known.
    pub input_per_mtok: Option<f64>,
    /// USD per million output tokens, when known.
    pub output_per_mtok: Option<f64>,
    /// Currency of the two rates above.
    pub currency: &'static str,
    /// The model the rates are for.
    pub model: &'static str,
    /// When the rates were last checked.
    pub as_of: &'static str,
    /// Where the rates came from.
    pub source: &'static str,
}

impl Pricing {
    /// Cost of a call, when the rates are known.
    pub fn cost(&self, input_tokens: u64, output_tokens: u64) -> Option<f64> {
        let inp = self.input_per_mtok?;
        let out = self.output_per_mtok?;
        Some((input_tokens as f64 * inp + output_tokens as f64 * out) / 1_000_000.0)
    }
}

/// A ready-to-use engine: model resolved, key present.
#[derive(Clone)]
pub struct EngineConfig {
    /// Which engine.
    pub engine: Engine,
    /// The resolved model id.
    pub model: String,
    api_key: String,
}

impl EngineConfig {
    /// Build one explicitly (tests, or a caller that already holds a key).
    pub fn new(engine: Engine, model: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            engine,
            model: model.into(),
            api_key: api_key.into(),
        }
    }

    /// The same credential, pointed at a different model.
    ///
    /// The judge runs on the Anthropic credential but usually on a different
    /// model from the `claude` probe engine, and the key is private (and
    /// redacted in `Debug`) so it cannot be lifted back out — this is the
    /// supported way to retarget a resolved config.
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}

/// Redacts the key so a `{config:?}` can never leak a credential.
impl std::fmt::Debug for EngineConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EngineConfig")
            .field("engine", &self.engine)
            .field("model", &self.model)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

/// Whether an engine can be probed on this machine.
#[derive(Debug, Clone)]
pub enum EngineStatus {
    /// Key present; here is the config.
    Ready(EngineConfig),
    /// No key. The caller **must** print this — see the module docs.
    MissingKey {
        /// Which engine was skipped.
        engine: Engine,
        /// Which variable to set.
        env: &'static str,
    },
}

impl EngineStatus {
    /// Resolve one engine from the environment.
    ///
    /// Provider keys may come from a repository-root `.env`; see
    /// [`crate::config::init_env`] for the (non-overriding) precedence rule.
    pub fn from_env(engine: Engine) -> Self {
        crate::config::init_env();

        let model = std::env::var(engine.model_env())
            .ok()
            .map(|m| m.trim().to_string())
            .filter(|m| !m.is_empty())
            .unwrap_or_else(|| engine.default_model().to_string());

        match std::env::var(engine.key_env()) {
            Ok(key) if !key.trim().is_empty() => {
                EngineStatus::Ready(EngineConfig::new(engine, model, key.trim()))
            }
            _ => EngineStatus::MissingKey {
                engine,
                env: engine.key_env(),
            },
        }
    }

    /// The engine this status is about.
    pub fn engine(&self) -> Engine {
        match self {
            Self::Ready(c) => c.engine,
            Self::MissingKey { engine, .. } => *engine,
        }
    }
}

/// One engine's answer to one prompt, normalised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAnswer {
    /// Full response text, verbatim.
    pub text: String,
    /// URLs the engine cited, vendor-native where available plus any URL the
    /// model wrote into its prose.
    pub cited_urls: Vec<String>,
    /// The model id that actually answered.
    pub model: String,
    /// Input tokens billed, when the vendor reports them.
    pub input_tokens: Option<u64>,
    /// Output tokens billed, when the vendor reports them.
    pub output_tokens: Option<u64>,
}

/// What happened to one probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeOutcome {
    /// The engine answered.
    Answered(Box<ProbeAnswer>),
    /// The engine did not, after retries. Counted and reported, never fatal.
    Failed {
        /// Human-readable reason, safe to print (never contains the key).
        reason: String,
        /// How many attempts were made.
        attempts: u32,
    },
}

impl ProbeOutcome {
    /// The answer, if there is one.
    pub fn answer(&self) -> Option<&ProbeAnswer> {
        match self {
            Self::Answered(a) => Some(a),
            Self::Failed { .. } => None,
        }
    }
}

/// Live HTTPS client for the four vendors.
#[derive(Debug, Clone)]
pub struct LlmClient {
    http: reqwest::Client,
}

impl Default for LlmClient {
    fn default() -> Self {
        Self::new()
    }
}

impl LlmClient {
    /// Build a client with the shared timeout.
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::builder()
                .timeout(REQUEST_TIMEOUT)
                .build()
                .unwrap_or_default(),
        }
    }

    /// Ask one engine one question, with backoff.
    pub async fn ask(&self, config: &EngineConfig, prompt: &str) -> ProbeOutcome {
        let mut last = String::from("no attempt was made");
        for attempt in 1..=MAX_ATTEMPTS {
            match self.attempt(config, prompt).await {
                Attempt::Ok(answer) => return ProbeOutcome::Answered(Box::new(answer)),
                Attempt::Fatal(reason) => {
                    return ProbeOutcome::Failed { reason, attempts: attempt };
                }
                Attempt::Retryable { reason, after } => {
                    last = reason;
                    if attempt == MAX_ATTEMPTS {
                        break;
                    }
                    tokio::time::sleep(after.unwrap_or_else(|| backoff_for(attempt))).await;
                }
            }
        }
        ProbeOutcome::Failed {
            reason: last,
            attempts: MAX_ATTEMPTS,
        }
    }

    async fn attempt(&self, config: &EngineConfig, prompt: &str) -> Attempt {
        let (url, body, headers) = request_for(config, prompt);
        let mut req = self.http.post(url).json(&body);
        for (name, value) in headers {
            req = req.header(name, value);
        }

        let resp = match req.send().await {
            Ok(r) => r,
            // A transport error is always worth one more go: DNS blips and
            // connection resets are far more common than a permanently
            // unreachable vendor.
            Err(e) => {
                return Attempt::Retryable {
                    reason: format!("transport error: {e}"),
                    after: None,
                };
            }
        };

        let status = resp.status();
        let retry_after = resp
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.trim().parse::<u64>().ok())
            .map(|secs| Duration::from_secs(secs).min(BACKOFF_CAP));
        let text = resp.text().await.unwrap_or_default();

        if status.as_u16() == 429 || status.is_server_error() {
            return Attempt::Retryable {
                reason: format!("HTTP {status}: {}", truncate(&text, 300)),
                after: retry_after,
            };
        }
        if !status.is_success() {
            // 400/401/403/404 are authoring or credential faults. Retrying
            // just multiplies the same error across the sweep.
            return Attempt::Fatal(format!("HTTP {status}: {}", truncate(&text, 300)));
        }

        let value: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => return Attempt::Fatal(format!("response was not JSON: {e}")),
        };
        match parse_answer(config, &value) {
            Ok(answer) => Attempt::Ok(answer),
            Err(reason) => Attempt::Fatal(reason),
        }
    }
}

enum Attempt {
    Ok(ProbeAnswer),
    Retryable { reason: String, after: Option<Duration> },
    Fatal(String),
}

fn backoff_for(attempt: u32) -> Duration {
    let scaled = BACKOFF_BASE.saturating_mul(1 << (attempt - 1).min(5));
    scaled.min(BACKOFF_CAP)
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.trim();
    if s.chars().count() <= max {
        return s.to_string();
    }
    s.chars().take(max).collect::<String>() + "…"
}

/// Build the vendor-specific request. Split out so the shapes are testable
/// without a network.
pub fn request_for(
    config: &EngineConfig,
    prompt: &str,
) -> (String, Value, Vec<(&'static str, String)>) {
    match config.engine {
        Engine::Chatgpt => (
            "https://api.openai.com/v1/chat/completions".to_string(),
            json!({
                "model": config.model,
                "max_completion_tokens": MAX_OUTPUT_TOKENS,
                "messages": [{"role": "user", "content": prompt}],
            }),
            vec![(
                "authorization",
                format!("Bearer {}", config.api_key),
            )],
        ),
        Engine::Perplexity => (
            "https://api.perplexity.ai/chat/completions".to_string(),
            json!({
                "model": config.model,
                "max_tokens": MAX_OUTPUT_TOKENS,
                "messages": [{"role": "user", "content": prompt}],
            }),
            vec![(
                "authorization",
                format!("Bearer {}", config.api_key),
            )],
        ),
        // No `temperature`/`top_p`/`top_k`: they are removed on current Claude
        // models and sending one is a 400. Thinking is left at the model's
        // default so the answer is the one a user would actually get.
        Engine::Claude => (
            "https://api.anthropic.com/v1/messages".to_string(),
            json!({
                "model": config.model,
                "max_tokens": MAX_OUTPUT_TOKENS,
                "messages": [{"role": "user", "content": prompt}],
            }),
            vec![
                ("x-api-key", config.api_key.clone()),
                ("anthropic-version", "2023-06-01".to_string()),
            ],
        ),
        Engine::Gemini => (
            format!(
                "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
                config.model
            ),
            json!({
                "contents": [{"role": "user", "parts": [{"text": prompt}]}],
                "generationConfig": {"maxOutputTokens": MAX_OUTPUT_TOKENS},
            }),
            vec![("x-goog-api-key", config.api_key.clone())],
        ),
    }
}

/// Normalise a vendor response into a [`ProbeAnswer`].
pub fn parse_answer(config: &EngineConfig, value: &Value) -> Result<ProbeAnswer, String> {
    let (text, mut cited, input_tokens, output_tokens, model) = match config.engine {
        Engine::Chatgpt | Engine::Perplexity => {
            let text = value
                .pointer("/choices/0/message/content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            // Perplexity returns its sources; OpenAI's chat completions do not.
            let cited = string_array(value.get("citations"))
                .into_iter()
                .chain(
                    value
                        .get("search_results")
                        .and_then(Value::as_array)
                        .map(|rows| {
                            rows.iter()
                                .filter_map(|r| r.get("url").and_then(Value::as_str))
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default(),
                )
                .collect::<Vec<_>>();
            (
                text,
                cited,
                value.pointer("/usage/prompt_tokens").and_then(Value::as_u64),
                value
                    .pointer("/usage/completion_tokens")
                    .and_then(Value::as_u64),
                value
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&config.model)
                    .to_string(),
            )
        }
        Engine::Claude => {
            let text = value
                .get("content")
                .and_then(Value::as_array)
                .map(|blocks| {
                    blocks
                        .iter()
                        .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                        .filter_map(|b| b.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            (
                text,
                Vec::new(),
                value.pointer("/usage/input_tokens").and_then(Value::as_u64),
                value.pointer("/usage/output_tokens").and_then(Value::as_u64),
                value
                    .get("model")
                    .and_then(Value::as_str)
                    .unwrap_or(&config.model)
                    .to_string(),
            )
        }
        Engine::Gemini => {
            let text = value
                .pointer("/candidates/0/content/parts")
                .and_then(Value::as_array)
                .map(|parts| {
                    parts
                        .iter()
                        .filter_map(|p| p.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("")
                })
                .unwrap_or_default();
            let cited = value
                .pointer("/candidates/0/groundingMetadata/groundingChunks")
                .and_then(Value::as_array)
                .map(|chunks| {
                    chunks
                        .iter()
                        .filter_map(|c| c.pointer("/web/uri").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            (
                text,
                cited,
                value
                    .pointer("/usageMetadata/promptTokenCount")
                    .and_then(Value::as_u64),
                value
                    .pointer("/usageMetadata/candidatesTokenCount")
                    .and_then(Value::as_u64),
                value
                    .get("modelVersion")
                    .and_then(Value::as_str)
                    .unwrap_or(&config.model)
                    .to_string(),
            )
        }
    };

    if text.trim().is_empty() {
        return Err(format!(
            "{} returned no text (a truncated or filtered answer would otherwise score as \
             'did not mention us')",
            config.engine
        ));
    }

    cited.extend(urls_in(&text));
    dedupe(&mut cited);

    Ok(ProbeAnswer {
        text,
        cited_urls: cited,
        model,
        input_tokens,
        output_tokens,
    })
}

fn string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn dedupe(urls: &mut Vec<String>) {
    let mut seen: Vec<String> = Vec::new();
    urls.retain(|u| {
        if seen.contains(u) {
            false
        } else {
            seen.push(u.clone());
            true
        }
    });
}

/// Every `http(s)://` URL written into the prose.
///
/// Models routinely name a source without the API exposing it as a citation;
/// dropping those would under-count exactly the third-party pages that shape a
/// wrong narrative. Trailing punctuation and markdown-link brackets are
/// stripped so `(https://x/y).` and `https://x/y` are the same source.
pub fn urls_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("http") {
        let candidate = &rest[at..];
        if !(candidate.starts_with("http://") || candidate.starts_with("https://")) {
            rest = &rest[at + 4..];
            continue;
        }
        let end = candidate
            .find(|c: char| c.is_whitespace() || c == '"' || c == '<' || c == '>')
            .unwrap_or(candidate.len());
        let url = candidate[..end].trim_end_matches(['.', ',', ')', ']', '}', ';', ':', '\'']);
        if url.len() > "https://".len() {
            out.push(url.to_string());
        }
        rest = &candidate[end..];
    }
    dedupe(&mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(engine: Engine) -> EngineConfig {
        EngineConfig::new(engine, engine.default_model(), "test-key")
    }

    #[test]
    fn engines_round_trip_through_the_contract_strings() {
        for engine in Engine::ALL {
            assert_eq!(Engine::parse(engine.as_str()), Some(engine));
        }
        assert_eq!(Engine::parse("bard"), None);
    }

    #[test]
    fn every_engine_declares_a_key_and_a_model_override() {
        for engine in Engine::ALL {
            assert!(engine.key_env().ends_with("_API_KEY"), "{engine}");
            assert!(engine.model_env().starts_with("GEO_"), "{engine}");
            assert!(!engine.default_model().is_empty(), "{engine}");
        }
    }

    #[test]
    fn a_missing_key_is_a_status_not_a_zero() {
        // The one behaviour that keeps a skipped engine from reading as lost
        // share downstream.
        let status = EngineStatus::MissingKey {
            engine: Engine::Gemini,
            env: "GEMINI_API_KEY",
        };
        assert_eq!(status.engine(), Engine::Gemini);
        assert!(matches!(status, EngineStatus::MissingKey { .. }));
    }

    #[test]
    fn a_config_never_debug_prints_its_key() {
        let printed = format!("{:?}", config(Engine::Claude));
        assert!(!printed.contains("test-key"), "{printed}");
        assert!(printed.contains("<redacted>"), "{printed}");
    }

    #[test]
    fn anthropic_requests_carry_the_version_header_and_no_sampling_params() {
        let (url, body, headers) = request_for(&config(Engine::Claude), "hi");
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert!(headers.iter().any(|(k, v)| *k == "anthropic-version" && v == "2023-06-01"));
        assert!(headers.iter().any(|(k, _)| *k == "x-api-key"));
        // temperature/top_p/top_k are removed on current Claude models: sending
        // one is a 400, which would fail the whole sweep on the claude engine.
        for banned in ["temperature", "top_p", "top_k"] {
            assert!(body.get(banned).is_none(), "{banned} must not be sent");
        }
        assert_eq!(body["max_tokens"], MAX_OUTPUT_TOKENS);
    }

    #[test]
    fn gemini_puts_the_key_in_a_header_not_the_query_string() {
        // A key in the URL lands in every proxy and access log in the path.
        let (url, _, headers) = request_for(&config(Engine::Gemini), "hi");
        assert!(!url.contains("test-key"), "{url}");
        assert!(headers.iter().any(|(k, _)| *k == "x-goog-api-key"));
    }

    #[test]
    fn openai_and_perplexity_use_bearer_auth() {
        for engine in [Engine::Chatgpt, Engine::Perplexity] {
            let (_, _, headers) = request_for(&config(engine), "hi");
            assert!(
                headers
                    .iter()
                    .any(|(k, v)| *k == "authorization" && v.starts_with("Bearer ")),
                "{engine}"
            );
        }
    }

    #[test]
    fn an_openai_response_parses() {
        let value = json!({
            "model": "gpt-5-2026-01-01",
            "choices": [{"message": {"content": "Try Mem0 or Zep."}}],
            "usage": {"prompt_tokens": 12, "completion_tokens": 34},
        });
        let answer = parse_answer(&config(Engine::Chatgpt), &value).expect("parses");
        assert_eq!(answer.text, "Try Mem0 or Zep.");
        assert_eq!(answer.input_tokens, Some(12));
        assert_eq!(answer.output_tokens, Some(34));
        assert_eq!(answer.model, "gpt-5-2026-01-01");
    }

    #[test]
    fn an_anthropic_response_joins_only_its_text_blocks() {
        let value = json!({
            "model": "claude-opus-5",
            "content": [
                {"type": "thinking", "thinking": ""},
                {"type": "text", "text": "AllSource is an event store."},
            ],
            "usage": {"input_tokens": 5, "output_tokens": 6},
        });
        let answer = parse_answer(&config(Engine::Claude), &value).expect("parses");
        assert_eq!(answer.text, "AllSource is an event store.");
    }

    #[test]
    fn a_gemini_response_takes_its_grounding_urls() {
        let value = json!({
            "modelVersion": "gemini-2.5-pro",
            "candidates": [{
                "content": {"parts": [{"text": "Zep is popular."}]},
                "groundingMetadata": {"groundingChunks": [
                    {"web": {"uri": "https://example.com/a"}},
                ]},
            }],
            "usageMetadata": {"promptTokenCount": 7, "candidatesTokenCount": 8},
        });
        let answer = parse_answer(&config(Engine::Gemini), &value).expect("parses");
        assert_eq!(answer.cited_urls, vec!["https://example.com/a"]);
        assert_eq!(answer.output_tokens, Some(8));
    }

    #[test]
    fn a_perplexity_response_takes_citations_and_search_results() {
        let value = json!({
            "choices": [{"message": {"content": "See the docs."}}],
            "citations": ["https://a.example/1"],
            "search_results": [{"url": "https://b.example/2"}],
        });
        let answer = parse_answer(&config(Engine::Perplexity), &value).expect("parses");
        assert_eq!(
            answer.cited_urls,
            vec!["https://a.example/1", "https://b.example/2"]
        );
    }

    #[test]
    fn an_empty_answer_is_an_error_not_a_silent_non_mention() {
        // A truncated or filtered answer scored as "did not mention us" would
        // be a false negative on the headline metric.
        let value = json!({"choices": [{"message": {"content": "   "}}]});
        let err = parse_answer(&config(Engine::Chatgpt), &value).expect_err("empty is an error");
        assert!(err.contains("no text"), "{err}");
    }

    #[test]
    fn prose_urls_are_recovered_and_deduped() {
        let text = "See https://www.all-source.xyz/docs and (https://www.all-source.xyz/docs). \
                    Also https://news.example/post-1.";
        let urls = urls_in(text);
        assert_eq!(
            urls,
            vec![
                "https://www.all-source.xyz/docs",
                "https://news.example/post-1",
            ]
        );
    }

    #[test]
    fn a_non_url_http_word_is_not_a_url() {
        assert!(urls_in("the http protocol is fine").is_empty());
    }

    #[test]
    fn backoff_grows_and_is_capped() {
        assert!(backoff_for(2) > backoff_for(1));
        assert!(backoff_for(20) <= BACKOFF_CAP);
    }

    #[test]
    fn only_anthropic_pricing_is_claimed() {
        // A fabricated price on a spend report is worse than no price.
        assert!(Engine::Claude.pricing().input_per_mtok.is_some());
        for engine in [Engine::Chatgpt, Engine::Gemini, Engine::Perplexity] {
            assert!(engine.pricing().input_per_mtok.is_none(), "{engine}");
            assert!(engine.pricing().cost(1000, 1000).is_none(), "{engine}");
        }
        assert_eq!(
            Engine::Claude.pricing().cost(1_000_000, 1_000_000),
            Some(30.0)
        );
    }
}
