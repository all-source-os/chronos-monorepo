use std::sync::OnceLock;

use crate::error::GeoError;

/// The Control Plane gateway. GEO tooling never talks to Core directly —
/// public auth terminates at the gateway.
pub const DEFAULT_API_URL: &str = "https://api.all-source.xyz";

/// Environment variable holding the gateway base URL.
pub const ENV_API_URL: &str = "ALLSOURCE_API_URL";

/// Environment variable holding the gateway API key.
pub const ENV_API_KEY: &str = "ALLSOURCE_API_KEY";

/// What happened when the process looked for a `.env` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DotenvOutcome {
    /// A `.env` was found and its entries were offered to the environment.
    Loaded {
        /// Absolute path of the file that was read.
        path: String,
    },
    /// No `.env` anywhere up the tree. Not an error: the keyless dry-run path
    /// and CI both run this way.
    NotFound,
    /// A `.env` exists but could not be read or parsed. Reported, never fatal —
    /// a malformed `.env` must not take down a run whose keys are already in
    /// the real environment.
    Unreadable {
        /// Why it could not be used.
        reason: String,
    },
}

/// The single `.env` load site for the whole of `geo-core`.
static DOTENV: OnceLock<DotenvOutcome> = OnceLock::new();

/// Load `.env` once, before any variable is read.
///
/// **Real process environment variables always win.** The loader below sets
/// only variables that are not already present, and it is the *only* dotenvy
/// call in the crate — there is deliberately no overriding variant anywhere,
/// and a test asserts that. Getting this backwards would let a stale `.env`
/// silently shadow a deliberate inline override (`ANTHROPIC_API_KEY=... geo
/// probe`) during a live sweep, which is exactly the kind of failure that
/// wastes an afternoon and a sweep's worth of spend.
///
/// dotenvy searches upward from the current directory, so the repository-root
/// `.env` is found whether you run from the repo root or from `tooling/geo`.
///
/// Idempotent: the first call does the work, later calls return the same
/// outcome. Callers may invoke it freely; the CLI calls it once at startup so
/// the outcome can be printed.
pub fn init_env() -> &'static DotenvOutcome {
    DOTENV.get_or_init(|| match dotenvy::dotenv() {
        Ok(path) => DotenvOutcome::Loaded {
            path: path.display().to_string(),
        },
        Err(e) if e.not_found() => DotenvOutcome::NotFound,
        Err(e) => DotenvOutcome::Unreadable {
            reason: e.to_string(),
        },
    })
}

/// Where GEO telemetry is written and with what credential.
#[derive(Clone)]
pub struct GeoConfig {
    api_url: String,
    api_key: String,
}

impl GeoConfig {
    /// Build a config explicitly (tests, or a caller that already has a key).
    pub fn new(api_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            api_url: api_url.into(),
            api_key: api_key.into(),
        }
    }

    /// Read `ALLSOURCE_API_URL` (optional) and `ALLSOURCE_API_KEY` (required).
    ///
    /// A missing key is a hard error with an actionable message — see
    /// [`GeoError::MissingApiKey`]. There is no silent no-op mode; use
    /// [`crate::EmitMode::DryRun`] when you deliberately do not want to write.
    pub fn from_env() -> Result<Self, GeoError> {
        // Cheap and idempotent, so config resolution is correct no matter
        // which entry point reached it.
        init_env();

        let api_url = match std::env::var(ENV_API_URL) {
            Ok(url) if url.trim().is_empty() => return Err(GeoError::EmptyApiUrl),
            Ok(url) => url.trim().trim_end_matches('/').to_string(),
            Err(_) => DEFAULT_API_URL.to_string(),
        };

        let api_key = std::env::var(ENV_API_KEY)
            .ok()
            .filter(|k| !k.trim().is_empty())
            .ok_or(GeoError::MissingApiKey)?;

        Ok(Self { api_url, api_key })
    }

    /// Gateway base URL (no trailing slash).
    pub fn api_url(&self) -> &str {
        &self.api_url
    }

    /// Gateway API key.
    pub fn api_key(&self) -> &str {
        &self.api_key
    }
}

/// Redacts the key so a `{config:?}` in a log line can never leak a credential.
impl std::fmt::Debug for GeoConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GeoConfig")
            .field("api_url", &self.api_url)
            .field("api_key", &"<redacted>")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_real_environment_variable_beats_a_dotenv_entry() {
        // The precedence that matters: `ANTHROPIC_API_KEY=... geo probe` must
        // win over a stale repository-root `.env`, or a live sweep silently
        // authenticates as the wrong thing.
        //
        // `CARGO_PKG_NAME` is guaranteed to already be in this process's real
        // environment (cargo sets it), so the test needs no `set_var` — which
        // is unsafe in this edition and denied workspace-wide.
        let dir = std::env::temp_dir().join(format!(
            "geo-dotenv-precedence-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        std::fs::create_dir_all(&dir).expect("temp dir");
        let path = dir.join(".env");
        std::fs::write(
            &path,
            "CARGO_PKG_NAME=value-from-dotenv\nGEO_DOTENV_TEST_ONLY=value-from-dotenv\n",
        )
        .expect("write .env");

        dotenvy::from_filename(&path).expect("the fixture .env loads");

        assert_eq!(
            std::env::var("CARGO_PKG_NAME").as_deref(),
            Ok(env!("CARGO_PKG_NAME")),
            "a .env entry overwrote a real process environment variable"
        );
        assert_eq!(
            std::env::var("GEO_DOTENV_TEST_ONLY").as_deref(),
            Ok("value-from-dotenv"),
            "a .env entry with no real counterpart was not applied"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn there_is_exactly_one_load_site_and_it_does_not_override() {
        // Two properties, asserted on the source because the difference is
        // invisible at runtime until the one run where it matters:
        //   1. `dotenv()` is the non-overriding loader; every `*_override`
        //      variant would let a stale .env shadow a real env var.
        //   2. Exactly one call site, in this file. Scattered loads are how
        //      load order becomes accidental.
        let source = include_str!("config.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        let calls: Vec<&str> = production
            .match_indices("dotenvy::")
            .map(|(i, _)| {
                let rest = &production[i..];
                &rest[..rest.len().min(24)]
            })
            .collect();
        assert_eq!(calls.len(), 1, "expected one dotenvy call, found {calls:?}");
        assert!(
            calls[0].starts_with("dotenvy::dotenv()"),
            "unexpected loader: {}",
            calls[0]
        );

        // ...and nowhere else in the crate.
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&src).expect("src dir") {
            let path = entry.expect("dir entry").path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs")
                || path.file_name().and_then(|n| n.to_str()) == Some("config.rs")
            {
                continue;
            }
            let body = std::fs::read_to_string(&path).expect("read source");
            assert!(
                !body.contains("dotenvy"),
                "{} calls dotenvy directly; go through config::init_env instead",
                path.display()
            );
        }
    }

    #[test]
    fn a_missing_dotenv_is_not_an_error() {
        // The keyless dry-run path and CI both run with no .env at all.
        let outcome = init_env();
        assert!(
            matches!(
                outcome,
                DotenvOutcome::Loaded { .. } | DotenvOutcome::NotFound
            ),
            "{outcome:?}"
        );
    }

}
