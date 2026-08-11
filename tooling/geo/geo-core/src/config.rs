use crate::error::GeoError;

/// The Control Plane gateway. GEO tooling never talks to Core directly —
/// public auth terminates at the gateway.
pub const DEFAULT_API_URL: &str = "https://api.all-source.xyz";

/// Environment variable holding the gateway base URL.
pub const ENV_API_URL: &str = "ALLSOURCE_API_URL";

/// Environment variable holding the gateway API key.
pub const ENV_API_KEY: &str = "ALLSOURCE_API_KEY";

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
