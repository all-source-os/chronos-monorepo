//! Ollama LLM backend for index compression.
//!
//! Implements [`LlmBackend`] using the Ollama generate API. Reuses a single
//! `reqwest::Client` across calls for connection pooling.

use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin};

use super::types::LlmBackend;

// =============================================================================
// Ollama request/response types
// =============================================================================

/// Ollama generate API request.
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

/// Ollama generate API response.
#[derive(Debug, Deserialize)]
struct OllamaResponse {
    response: String,
}

// =============================================================================
// OllamaBackend
// =============================================================================

/// LLM backend using the Ollama API.
///
/// Reuses a single `reqwest::Client` across calls for connection pooling
/// and avoids per-request client construction overhead.
pub struct OllamaBackend {
    client: reqwest::Client,
    endpoint: String,
    model: String,
}

impl OllamaBackend {
    /// Create a new Ollama backend.
    ///
    /// # Arguments
    /// * `endpoint` — The Ollama API URL (e.g. `http://localhost:11434/api/generate`).
    /// * `model` — The model name (e.g. `"mistral"`, `"mistral:7b"`).
    pub fn new(endpoint: String, model: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("failed to build reqwest client");

        Self {
            client,
            endpoint,
            model,
        }
    }
}

impl LlmBackend for OllamaBackend {
    fn generate(
        &self,
        prompt: &str,
    ) -> Pin<Box<dyn Future<Output = std::result::Result<String, String>> + Send + '_>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        Box::pin(async move {
            let response = self
                .client
                .post(&self.endpoint)
                .json(&request)
                .send()
                .await
                .map_err(|e| format!("LLM request failed: {e}"))?;

            if !response.status().is_success() {
                return Err(format!("LLM returned status {}", response.status()));
            }

            let body: OllamaResponse = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse LLM response: {e}"))?;

            Ok(body.response)
        })
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_backend_construction() {
        let backend = OllamaBackend::new(
            "http://localhost:11434/api/generate".to_string(),
            "mistral".to_string(),
        );
        assert_eq!(backend.endpoint, "http://localhost:11434/api/generate");
        assert_eq!(backend.model, "mistral");
    }

    #[tokio::test]
    async fn test_ollama_backend_unreachable_returns_error() {
        let backend = OllamaBackend::new(
            "http://localhost:99999/api/generate".to_string(),
            "mistral".to_string(),
        );

        let result = backend.generate("test prompt").await;
        assert!(result.is_err());
    }
}
