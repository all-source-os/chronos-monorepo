//! AI Workflow Projection Templates.
//!
//! Pre-built projections for common AI agent patterns:
//! - Token usage / cost tracking
//! - MCP tool call audit (success rate, latency)
//! - Human-in-the-loop approval queue
//! - Agent utilization (active/idle/capacity)

use crate::{application::services::projection::Projection, domain::entities::Event, error::Result};
use dashmap::DashMap;
use serde_json::{json, Value};
use std::sync::Arc;

// =============================================================================
// Token Usage / Cost Tracking Projection
// =============================================================================

/// Tracks LLM token usage and costs per entity, with per-model breakdown.
///
/// Folds `llm.call.completed` events into aggregate stats:
/// `{ total_input_tokens, total_output_tokens, total_cost_usd, calls_count, by_model }`
pub struct TokenUsageProjection {
    states: Arc<DashMap<String, Value>>,
}

impl TokenUsageProjection {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
    }
}

impl Projection for TokenUsageProjection {
    fn name(&self) -> &str {
        "token_usage"
    }

    fn process(&self, event: &Event) -> Result<()> {
        if event.event_type_str() != "llm.call.completed" {
            return Ok(());
        }

        let entity_id = event.entity_id_str().to_string();
        let payload = &event.payload;

        let input_tokens = payload.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let output_tokens = payload.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0);
        let cost_usd = payload.get("cost_usd").and_then(|v| v.as_f64()).unwrap_or(0.0);
        let model = payload
            .get("model")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        self.states
            .entry(entity_id)
            .and_modify(|state| {
                let ti = state["total_input_tokens"].as_u64().unwrap_or(0) + input_tokens;
                let to = state["total_output_tokens"].as_u64().unwrap_or(0) + output_tokens;
                let tc = state["total_cost_usd"].as_f64().unwrap_or(0.0) + cost_usd;
                let cc = state["calls_count"].as_u64().unwrap_or(0) + 1;

                state["total_input_tokens"] = json!(ti);
                state["total_output_tokens"] = json!(to);
                state["total_cost_usd"] = json!(tc);
                state["calls_count"] = json!(cc);

                // Per-model breakdown
                if let Some(by_model) = state.get_mut("by_model") {
                    let model_entry = by_model
                        .as_object_mut()
                        .unwrap()
                        .entry(&model)
                        .or_insert(json!({"calls": 0, "input_tokens": 0, "output_tokens": 0, "cost_usd": 0.0}));
                    model_entry["calls"] = json!(model_entry["calls"].as_u64().unwrap_or(0) + 1);
                    model_entry["input_tokens"] =
                        json!(model_entry["input_tokens"].as_u64().unwrap_or(0) + input_tokens);
                    model_entry["output_tokens"] =
                        json!(model_entry["output_tokens"].as_u64().unwrap_or(0) + output_tokens);
                    model_entry["cost_usd"] =
                        json!(model_entry["cost_usd"].as_f64().unwrap_or(0.0) + cost_usd);
                }
            })
            .or_insert_with(|| {
                json!({
                    "total_input_tokens": input_tokens,
                    "total_output_tokens": output_tokens,
                    "total_cost_usd": cost_usd,
                    "calls_count": 1,
                    "by_model": {
                        model: {
                            "calls": 1,
                            "input_tokens": input_tokens,
                            "output_tokens": output_tokens,
                            "cost_usd": cost_usd,
                        }
                    }
                })
            });

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// MCP Tool Call Audit Projection
// =============================================================================

/// Tracks MCP tool call success rates and latency per entity.
///
/// Folds `mcp.tool.result` and `mcp.tool.error` events into per-tool stats.
pub struct ToolCallAuditProjection {
    states: Arc<DashMap<String, Value>>,
}

impl ToolCallAuditProjection {
    pub fn new() -> Self {
        Self {
            states: Arc::new(DashMap::new()),
        }
    }
}

impl Projection for ToolCallAuditProjection {
    fn name(&self) -> &str {
        "tool_call_audit"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let event_type = event.event_type_str();
        if event_type != "mcp.tool.result" && event_type != "mcp.tool.error" {
            return Ok(());
        }

        let entity_id = event.entity_id_str().to_string();
        let payload = &event.payload;
        let tool_name = payload
            .get("tool_name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let is_success = event_type == "mcp.tool.result";
        let duration_ms = payload.get("duration_ms").and_then(|v| v.as_f64());

        self.states
            .entry(entity_id)
            .and_modify(|state| {
                let tool = state
                    .as_object_mut()
                    .unwrap()
                    .entry(&tool_name)
                    .or_insert(json!({
                        "total_calls": 0,
                        "successes": 0,
                        "failures": 0,
                        "success_rate": 0.0,
                        "durations": [],
                    }));
                let total = tool["total_calls"].as_u64().unwrap_or(0) + 1;
                let successes = tool["successes"].as_u64().unwrap_or(0)
                    + if is_success { 1 } else { 0 };
                let failures = tool["failures"].as_u64().unwrap_or(0)
                    + if is_success { 0 } else { 1 };

                tool["total_calls"] = json!(total);
                tool["successes"] = json!(successes);
                tool["failures"] = json!(failures);
                tool["success_rate"] = json!(successes as f64 / total as f64);

                if let Some(d) = duration_ms {
                    if let Some(durations) = tool["durations"].as_array_mut() {
                        durations.push(json!(d));
                        let mut sorted: Vec<f64> = durations
                            .iter()
                            .filter_map(|v| v.as_f64())
                            .collect();
                        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
                        let len = sorted.len();
                        let p50_idx = len / 2;
                        let p95_idx = ((len as f64 * 0.95).ceil() as usize).min(len - 1);
                        tool["p50_ms"] = json!(sorted[p50_idx]);
                        tool["p95_ms"] = json!(sorted[p95_idx]);
                    }
                }
            })
            .or_insert_with(|| {
                let mut tool_state = json!({
                    "total_calls": 1,
                    "successes": if is_success { 1 } else { 0 },
                    "failures": if is_success { 0 } else { 1 },
                    "success_rate": if is_success { 1.0 } else { 0.0 },
                    "durations": [],
                });
                if let Some(d) = duration_ms {
                    tool_state["durations"] = json!([d]);
                    tool_state["p50_ms"] = json!(d);
                    tool_state["p95_ms"] = json!(d);
                }
                json!({ tool_name: tool_state })
            });

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        self.states.get(entity_id).map(|v| v.clone())
    }

    fn clear(&self) {
        self.states.clear();
    }
}

// =============================================================================
// Human-in-the-Loop Queue Projection
// =============================================================================

/// Tracks pending approval requests across all workflows.
///
/// Query with entity_id `__all` to get the full pending queue.
pub struct HumanInLoopQueueProjection {
    /// entity_id -> (reason, timestamp) for pending approvals
    pending: Arc<DashMap<String, (String, chrono::DateTime<chrono::Utc>)>>,
}

impl HumanInLoopQueueProjection {
    pub fn new() -> Self {
        Self {
            pending: Arc::new(DashMap::new()),
        }
    }
}

impl Projection for HumanInLoopQueueProjection {
    fn name(&self) -> &str {
        "human_in_loop_queue"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();

        match event_type {
            "workflow.approval.requested" => {
                let reason = event
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                self.pending.insert(entity_id, (reason, event.timestamp()));
            }
            "workflow.approval.granted" | "workflow.approval.rejected" => {
                self.pending.remove(&entity_id);
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        if entity_id == "__all" {
            let mut entries: Vec<(String, String, chrono::DateTime<chrono::Utc>)> = self
                .pending
                .iter()
                .map(|entry| {
                    let (reason, ts) = entry.value();
                    (entry.key().clone(), reason.clone(), *ts)
                })
                .collect();
            // Sort by timestamp (oldest first)
            entries.sort_by_key(|(_, _, ts)| *ts);

            let pending_approvals: Vec<Value> = entries
                .into_iter()
                .map(|(eid, reason, ts)| {
                    json!({
                        "entity_id": eid,
                        "reason": reason,
                        "requested_at": ts.to_rfc3339(),
                    })
                })
                .collect();

            Some(json!({ "pending_approvals": pending_approvals }))
        } else {
            self.pending.get(entity_id).map(|entry| {
                let (reason, ts) = entry.value();
                json!({
                    "entity_id": entity_id,
                    "reason": reason,
                    "requested_at": ts.to_rfc3339(),
                })
            })
        }
    }

    fn clear(&self) {
        self.pending.clear();
    }
}

// =============================================================================
// Agent Utilization Projection
// =============================================================================

/// Tracks agent (replicant) utilization: active, idle, total capacity.
///
/// Folds `replicant.*` and `workflow.claimed`/`workflow.output.ready` events.
/// Query with entity_id `__all` to get aggregate utilization.
pub struct AgentUtilizationProjection {
    /// replicant_id -> status ("active"/"idle"/"stale")
    replicants: Arc<DashMap<String, String>>,
    /// replicant_id -> set of active workflow IDs
    active_workflows: Arc<DashMap<String, Vec<String>>>,
    /// workflow_id -> replicant_id (for mapping completions back)
    workflow_to_replicant: Arc<DashMap<String, String>>,
}

impl AgentUtilizationProjection {
    pub fn new() -> Self {
        Self {
            replicants: Arc::new(DashMap::new()),
            active_workflows: Arc::new(DashMap::new()),
            workflow_to_replicant: Arc::new(DashMap::new()),
        }
    }
}

impl Projection for AgentUtilizationProjection {
    fn name(&self) -> &str {
        "agent_utilization"
    }

    fn process(&self, event: &Event) -> Result<()> {
        let event_type = event.event_type_str();
        let entity_id = event.entity_id_str().to_string();

        match event_type {
            "replicant.registered" => {
                self.replicants.insert(entity_id.clone(), "idle".to_string());
                self.active_workflows.insert(entity_id, Vec::new());
            }
            "replicant.stale" => {
                self.replicants.insert(entity_id, "stale".to_string());
            }
            "replicant.heartbeat" => {
                // Only reactivate if not stale
                self.replicants.entry(entity_id).and_modify(|status| {
                    if status != "stale" {
                        *status = "idle".to_string();
                    }
                });
            }
            "workflow.claimed" => {
                if let Some(rid) = event.payload.get("replicant_id").and_then(|v| v.as_str()) {
                    let rid = rid.to_string();
                    self.workflow_to_replicant
                        .insert(entity_id.clone(), rid.clone());
                    self.active_workflows
                        .entry(rid.clone())
                        .and_modify(|wfs| {
                            if !wfs.contains(&entity_id) {
                                wfs.push(entity_id.clone());
                            }
                        });
                    self.replicants.entry(rid).and_modify(|status| {
                        *status = "active".to_string();
                    });
                }
            }
            "workflow.output.ready" | "workflow.step.failed" => {
                // Workflow completed — free up the replicant
                if let Some((_, rid)) = self.workflow_to_replicant.remove(&entity_id) {
                    self.active_workflows.entry(rid.clone()).and_modify(|wfs| {
                        wfs.retain(|w| w != &entity_id);
                    });
                    // If no more active workflows, mark as idle
                    if let Some(wfs) = self.active_workflows.get(&rid) {
                        if wfs.is_empty() {
                            self.replicants.entry(rid).and_modify(|status| {
                                *status = "idle".to_string();
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn get_state(&self, entity_id: &str) -> Option<Value> {
        if entity_id == "__all" {
            let mut active = 0u64;
            let mut idle = 0u64;
            let mut total = 0u64;

            for entry in self.replicants.iter() {
                let status = entry.value();
                if status != "stale" {
                    total += 1;
                    match status.as_str() {
                        "active" => active += 1,
                        "idle" => idle += 1,
                        _ => {}
                    }
                }
            }

            Some(json!({
                "total_capacity": total,
                "active": active,
                "idle": idle,
            }))
        } else {
            self.replicants.get(entity_id).map(|status| {
                json!({ "status": status.value() })
            })
        }
    }

    fn clear(&self) {
        self.replicants.clear();
        self.active_workflows.clear();
        self.workflow_to_replicant.clear();
    }
}
