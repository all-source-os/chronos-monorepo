pub mod backend;
pub mod config;
pub mod core_task_repo;
pub mod http_core_client;
pub mod id;
#[cfg(feature = "prime")]
pub mod prime_memory;
pub mod projection;
pub mod workspace;

/// Read the agent identifier from `CN_AGENT_ID` env var, defaulting to `"human"`.
pub fn agent_id() -> String {
    std::env::var("CN_AGENT_ID").unwrap_or_else(|_| "human".to_string())
}
