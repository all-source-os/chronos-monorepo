use allframe::hyper::HeaderMap;

#[derive(Clone)]
pub struct AuthConfig {
    pub deploy_token: String,
    pub registry_tokens: Vec<String>,
}

impl AuthConfig {
    pub fn from_env() -> Self {
        let deploy_token =
            std::env::var("DEPLOY_TOKEN").expect("DEPLOY_TOKEN must be set");
        let registry_tokens = std::env::var("REGISTRY_TOKENS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        Self {
            deploy_token,
            registry_tokens,
        }
    }

    pub fn is_public(&self) -> bool {
        self.registry_tokens.is_empty()
    }

    pub fn verify_deploy_token(&self, headers: &HeaderMap) -> bool {
        extract_bearer(headers)
            .map(|t| t == self.deploy_token)
            .unwrap_or(false)
    }

    pub fn verify_download_token(&self, headers: &HeaderMap) -> bool {
        if self.is_public() {
            return true;
        }
        extract_bearer(headers)
            .map(|t| self.registry_tokens.iter().any(|rt| rt == t))
            .unwrap_or(false)
    }
}

fn extract_bearer(headers: &HeaderMap) -> Option<&str> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
}
