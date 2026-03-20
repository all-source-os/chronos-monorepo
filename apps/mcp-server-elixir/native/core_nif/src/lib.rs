// rustler NIF macros generate code that conflicts with pedantic clippy lints:
// - non_local_definitions, unused_must_use: from rustler::resource! macro
// - elidable_lifetime_names: NIF fns need explicit lifetimes for Env<'a>/Term<'a>
// - needless_pass_by_value: NIF fns receive ResourceArc/Term by value (rustler ABI)
// - unnecessary_cast: macro-generated u64 as u64
// - uninlined_format_args: format strings in error paths
#![allow(
    non_local_definitions,
    unused_must_use,
    clippy::elidable_lifetime_names,
    clippy::needless_pass_by_value,
    clippy::unnecessary_cast,
    clippy::uninlined_format_args
)]

use allsource_core::embedded::{Config, EmbeddedCore, EventView, IngestEvent, Query};
use rustler::{Atom, Encoder, Env, NifResult, ResourceArc, Term};
use std::sync::Arc;
use tokio::runtime::Runtime;

mod atoms {
    rustler::atoms! {
        ok,
        error,
        not_found,
    }
}

struct CoreResource {
    core: Arc<EmbeddedCore>,
    rt: Arc<Runtime>,
}

#[rustler::nif]
fn nif_ping() -> String {
    "pong".to_string()
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_open<'a>(env: Env<'a>, config_map: Term<'a>) -> NifResult<Term<'a>> {
    let rt = Arc::new(
        Runtime::new().map_err(|e| rustler::Error::Term(Box::new(format!("runtime: {e}"))))?,
    );

    let mut builder = Config::builder();

    if let Ok(data_dir) = get_string(env, config_map, "data_dir") {
        if !data_dir.is_empty() {
            builder = builder.data_dir(data_dir);
        }
    }

    if let Ok(node_id) = get_u32(env, config_map, "node_id") {
        builder = builder.node_id(node_id);
    }

    if let Ok(ms) = get_u64(env, config_map, "wal_fsync_interval_ms") {
        builder = builder.wal_fsync_interval_ms(ms);
    }

    if let Ok(secs) = get_u64(env, config_map, "parquet_flush_interval_secs") {
        builder = builder.parquet_flush_interval_secs(secs);
    }

    let config = builder
        .build()
        .map_err(|e| rustler::Error::Term(Box::new(format!("config: {e}"))))?;

    let core = rt
        .block_on(EmbeddedCore::open(config))
        .map_err(|e| rustler::Error::Term(Box::new(format!("open: {e}"))))?;

    let resource = ResourceArc::new(CoreResource {
        core: Arc::new(core),
        rt,
    });
    Ok((atoms::ok(), resource).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_shutdown(resource: ResourceArc<CoreResource>) -> NifResult<Atom> {
    resource
        .rt
        .block_on(resource.core.shutdown())
        .map_err(|e| rustler::Error::Term(Box::new(format!("shutdown: {e}"))))?;
    Ok(atoms::ok())
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_query<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    params: Term<'a>,
) -> NifResult<Term<'a>> {
    let query = build_query(env, params)?;

    let events = resource
        .rt
        .block_on(resource.core.query(query))
        .map_err(|e| rustler::Error::Term(Box::new(format!("query: {e}"))))?;

    let events_list: Vec<_> = events.iter().map(|ev| encode_event(env, ev)).collect();
    let count = events.len();

    let result = Term::map_new(env);
    let result = result
        .map_put("events".encode(env), events_list.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put failed")))?;
    let result = result
        .map_put("count".encode(env), count.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put failed")))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_ingest<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    params: Term<'a>,
) -> NifResult<Term<'a>> {
    let entity_id = get_string(env, params, "entity_id")?;
    let event_type = get_string(env, params, "event_type")?;
    let payload_str =
        get_string(env, params, "payload").or_else(|_| get_json_string(env, params, "payload"))?;
    let payload: serde_json::Value = serde_json::from_str(&payload_str)
        .unwrap_or_else(|_| serde_json::Value::String(payload_str.clone()));

    let metadata_val = get_json_string(env, params, "metadata")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok());

    let event = IngestEvent {
        entity_id: &entity_id,
        event_type: &event_type,
        payload,
        metadata: metadata_val,
        tenant_id: None,
    };

    resource
        .rt
        .block_on(resource.core.ingest(event))
        .map_err(|e| rustler::Error::Term(Box::new(format!("ingest: {e}"))))?;

    let result = Term::map_new(env);
    let result = result
        .map_put("status".encode(env), "ok".encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put failed")))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_get_stats<'a>(env: Env<'a>, resource: ResourceArc<CoreResource>) -> NifResult<Term<'a>> {
    let stats = resource.core.stats();

    let result = Term::map_new(env);
    let result = result
        .map_put("total_events".encode(env), stats.total_events.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;
    let result = result
        .map_put(
            "total_entities".encode(env),
            stats.total_entities.encode(env),
        )
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;
    let result = result
        .map_put(
            "total_event_types".encode(env),
            stats.total_event_types.encode(env),
        )
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;
    let result = result
        .map_put(
            "total_ingested".encode(env),
            (stats.total_ingested as u64).encode(env),
        )
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_get_snapshot<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    entity_id: String,
) -> NifResult<Term<'a>> {
    match resource.core.projection("entity_snapshots", &entity_id) {
        Some(snapshot) => {
            let json_str = serde_json::to_string(&snapshot)
                .map_err(|e| rustler::Error::Term(Box::new(format!("json: {e}"))))?;
            Ok((atoms::ok(), json_str).encode(env))
        }
        None => Ok((atoms::error(), atoms::not_found()).encode(env)),
    }
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_reconstruct_state<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    entity_id: String,
    _opts: Term<'a>,
) -> NifResult<Term<'a>> {
    // Reconstruct by querying all events for entity and folding
    let query = Query::new().entity_id(&entity_id);

    let events = resource
        .rt
        .block_on(resource.core.query(query))
        .map_err(|e| rustler::Error::Term(Box::new(format!("query: {e}"))))?;

    if events.is_empty() {
        return Ok((atoms::error(), atoms::not_found()).encode(env));
    }

    // Fold events into a state map
    let mut state = serde_json::Map::new();
    state.insert(
        "entity_id".to_string(),
        serde_json::Value::String(entity_id.clone()),
    );
    state.insert(
        "event_count".to_string(),
        serde_json::Value::Number(events.len().into()),
    );

    if let Some(last) = events.last() {
        state.insert(
            "last_event_type".to_string(),
            serde_json::Value::String(last.event_type.clone()),
        );
        state.insert(
            "last_updated".to_string(),
            serde_json::Value::String(last.timestamp.to_rfc3339()),
        );

        // Merge last event's payload into state
        if let serde_json::Value::Object(payload_map) = &last.payload {
            for (k, v) in payload_map {
                state.insert(k.clone(), v.clone());
            }
        }
    }

    let state_json = serde_json::to_string(&serde_json::Value::Object(state))
        .map_err(|e| rustler::Error::Term(Box::new(format!("json: {e}"))))?;

    Ok((atoms::ok(), state_json).encode(env))
}

#[cfg(feature = "embedded-toon")]
#[rustler::nif(schedule = "DirtyCpu")]
fn nif_query_toon<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    params: Term<'a>,
) -> NifResult<Term<'a>> {
    let query = build_query(env, params)?;

    let toon = resource
        .rt
        .block_on(resource.core.query_toon(query))
        .map_err(|e| rustler::Error::Term(Box::new(format!("query_toon: {e}"))))?;

    Ok((atoms::ok(), toon).encode(env))
}

#[cfg(not(feature = "embedded-toon"))]
#[rustler::nif]
fn nif_query_toon<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _params: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((
        atoms::error(),
        "TOON format not available (feature disabled)",
    )
        .encode(env))
}

#[rustler::nif]
fn nif_semantic_search<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _params: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_hybrid_search<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _params: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_list_schemas<'a>(env: Env<'a>, _resource: ResourceArc<CoreResource>) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_register_schema<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _params: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_validate_schema<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _params: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_get_schema<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _event_type: String,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_compact_storage<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
) -> NifResult<Term<'a>> {
    // Trigger compaction via stats check (embedded mode doesn't have dedicated compaction endpoint)
    let _stats = resource.core.stats();
    Ok((atoms::ok(), "compaction not applicable in embedded mode").encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_storage_stats<'a>(env: Env<'a>, resource: ResourceArc<CoreResource>) -> NifResult<Term<'a>> {
    let ds = resource.core.durability_status();

    let result = Term::map_new(env);
    let result = map_put(env, result, "memory_events", ds.memory_events.encode(env))?;
    let result = map_put(
        env,
        result,
        "parquet_enabled",
        ds.parquet_enabled.encode(env),
    )?;
    let result = map_put(env, result, "parquet_files", ds.parquet_files.encode(env))?;
    let result = map_put(env, result, "parquet_bytes", ds.parquet_bytes.encode(env))?;
    let result = map_put(
        env,
        result,
        "parquet_pending_batch",
        ds.parquet_pending_batch.encode(env),
    )?;
    let result = map_put(env, result, "wal_entries", ds.wal_entries.encode(env))?;
    let result = map_put(env, result, "wal_bytes", ds.wal_bytes.encode(env))?;
    let result = map_put(env, result, "durable", ds.durable.encode(env))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_wal_status<'a>(env: Env<'a>, resource: ResourceArc<CoreResource>) -> NifResult<Term<'a>> {
    let ds = resource.core.durability_status();

    let result = Term::map_new(env);
    let result = map_put(env, result, "role", "embedded".encode(env))?;
    let result = map_put(env, result, "wal_enabled", ds.wal_enabled.encode(env))?;
    let result = map_put(env, result, "wal_entries", ds.wal_entries.encode(env))?;
    let result = map_put(env, result, "wal_bytes", ds.wal_bytes.encode(env))?;
    let result = map_put(env, result, "wal_sequence", ds.wal_sequence.encode(env))?;
    let result = map_put(env, result, "memory_events", ds.memory_events.encode(env))?;
    let result = map_put(env, result, "durable", ds.durable.encode(env))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_health_deep<'a>(env: Env<'a>, resource: ResourceArc<CoreResource>) -> NifResult<Term<'a>> {
    let ds = resource.core.durability_status();
    let status = if ds.warnings.is_empty() {
        "healthy"
    } else {
        "degraded"
    };

    let result = Term::map_new(env);
    let result = map_put(env, result, "status", status.encode(env))?;
    let result = map_put(env, result, "mode", "embedded".encode(env))?;
    let result = map_put(env, result, "memory_events", ds.memory_events.encode(env))?;
    let result = map_put(env, result, "wal_enabled", ds.wal_enabled.encode(env))?;
    let result = map_put(env, result, "wal_entries", ds.wal_entries.encode(env))?;
    let result = map_put(
        env,
        result,
        "parquet_enabled",
        ds.parquet_enabled.encode(env),
    )?;
    let result = map_put(env, result, "parquet_files", ds.parquet_files.encode(env))?;
    let result = map_put(env, result, "durable", ds.durable.encode(env))?;
    let warnings_list: Vec<_> = ds.warnings.iter().map(|w| w.as_str().encode(env)).collect();
    let result = map_put(env, result, "warnings", warnings_list.encode(env))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif(schedule = "DirtyCpu")]
fn nif_durability_status<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
) -> NifResult<Term<'a>> {
    let ds = resource.core.durability_status();

    let result = Term::map_new(env);
    let result = map_put(env, result, "memory_events", ds.memory_events.encode(env))?;
    let result = map_put(env, result, "wal_enabled", ds.wal_enabled.encode(env))?;
    let result = map_put(env, result, "wal_entries", ds.wal_entries.encode(env))?;
    let result = map_put(env, result, "wal_bytes", ds.wal_bytes.encode(env))?;
    let result = map_put(env, result, "wal_sequence", ds.wal_sequence.encode(env))?;
    let result = map_put(
        env,
        result,
        "parquet_enabled",
        ds.parquet_enabled.encode(env),
    )?;
    let result = map_put(env, result, "parquet_files", ds.parquet_files.encode(env))?;
    let result = map_put(env, result, "parquet_bytes", ds.parquet_bytes.encode(env))?;
    let result = map_put(
        env,
        result,
        "parquet_pending_batch",
        ds.parquet_pending_batch.encode(env),
    )?;
    let result = map_put(env, result, "durable", ds.durable.encode(env))?;
    let warnings_list: Vec<_> = ds.warnings.iter().map(|w| w.as_str().encode(env)).collect();
    let result = map_put(env, result, "warnings", warnings_list.encode(env))?;

    Ok((atoms::ok(), result).encode(env))
}

#[rustler::nif]
fn nif_analytics_frequency<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _opts: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_analytics_summary<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _opts: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[rustler::nif]
fn nif_analytics_correlation<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _opts: Term<'a>,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "not available in embedded mode").encode(env))
}

#[cfg(feature = "embedded-sync")]
#[rustler::nif(schedule = "DirtyCpu")]
fn nif_sync<'a>(
    env: Env<'a>,
    resource: ResourceArc<CoreResource>,
    remote_url: String,
    node_id: String,
) -> NifResult<Term<'a>> {
    use allsource_core::embedded::sync_transport::SyncClient;

    let client = SyncClient::new(remote_url, node_id);

    let stats = resource
        .rt
        .block_on(client.sync(&resource.core))
        .map_err(|e| rustler::Error::Term(Box::new(format!("sync: {e}"))))?;

    let result = Term::map_new(env);
    let result = result
        .map_put("pushed".encode(env), stats.pushed.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;
    let result = result
        .map_put("pulled".encode(env), stats.pulled.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;
    let result = result
        .map_put("conflicts".encode(env), stats.conflicts.encode(env))
        .map_err(|_| rustler::Error::Term(Box::new("map_put")))?;

    Ok((atoms::ok(), result).encode(env))
}

#[cfg(not(feature = "embedded-sync"))]
#[rustler::nif]
fn nif_sync<'a>(
    env: Env<'a>,
    _resource: ResourceArc<CoreResource>,
    _remote_url: String,
    _node_id: String,
) -> NifResult<Term<'a>> {
    Ok((atoms::error(), "sync not available (feature disabled)").encode(env))
}

// ============================================================================
// Helper functions
// ============================================================================

fn build_query<'a>(env: Env<'a>, params: Term<'a>) -> NifResult<Query> {
    let mut query = Query::new();

    if let Ok(entity_id) = get_string(env, params, "entity_id") {
        if !entity_id.is_empty() {
            query = query.entity_id(entity_id);
        }
    }

    if let Ok(event_type) = get_string(env, params, "event_type") {
        if !event_type.is_empty() {
            query = query.event_type(event_type);
        }
    }

    if let Ok(limit) = get_usize(env, params, "limit") {
        query = query.limit(limit);
    }

    Ok(query)
}

fn encode_event<'a>(env: Env<'a>, event: &EventView) -> Term<'a> {
    let mut map = Term::map_new(env);
    map = map
        .map_put("id".encode(env), event.id.to_string().encode(env))
        .unwrap();
    map = map
        .map_put(
            "event_type".encode(env),
            event.event_type.as_str().encode(env),
        )
        .unwrap();
    map = map
        .map_put(
            "entity_id".encode(env),
            event.entity_id.as_str().encode(env),
        )
        .unwrap();
    map = map
        .map_put(
            "tenant_id".encode(env),
            event.tenant_id.as_str().encode(env),
        )
        .unwrap();
    map = map
        .map_put(
            "payload".encode(env),
            serde_json::to_string(&event.payload)
                .unwrap_or_default()
                .encode(env),
        )
        .unwrap();
    map = map
        .map_put(
            "timestamp".encode(env),
            event.timestamp.to_rfc3339().encode(env),
        )
        .unwrap();
    map = map
        .map_put("version".encode(env), event.version.encode(env))
        .unwrap();

    if let Some(ref meta) = event.metadata {
        map = map
            .map_put(
                "metadata".encode(env),
                serde_json::to_string(meta).unwrap_or_default().encode(env),
            )
            .unwrap();
    }

    map
}

fn map_put<'a>(env: Env<'a>, map: Term<'a>, key: &str, value: Term<'a>) -> NifResult<Term<'a>> {
    map.map_put(key.encode(env), value)
        .map_err(|_| rustler::Error::Term(Box::new(format!("map_put failed for key: {key}"))))
}

fn get_string<'a>(env: Env<'a>, map: Term<'a>, key: &str) -> NifResult<String> {
    let key_term = key.encode(env);
    match map.map_get(key_term) {
        Ok(val) => {
            let s: String = val
                .decode()
                .map_err(|_| rustler::Error::Term(Box::new(format!("{key} must be a string"))))?;
            Ok(s)
        }
        Err(_) => Err(rustler::Error::Term(Box::new(format!(
            "missing key: {key}"
        )))),
    }
}

fn get_json_string<'a>(env: Env<'a>, map: Term<'a>, key: &str) -> NifResult<String> {
    let key_term = key.encode(env);
    match map.map_get(key_term) {
        Ok(val) => {
            // Try to decode as a string first
            if let Ok(s) = val.decode::<String>() {
                return Ok(s);
            }
            // For complex terms, try to represent as debug string
            Ok(format!("{:?}", val))
        }
        Err(_) => Err(rustler::Error::Term(Box::new(format!(
            "missing key: {key}"
        )))),
    }
}

fn get_u32<'a>(env: Env<'a>, map: Term<'a>, key: &str) -> NifResult<u32> {
    let key_term = key.encode(env);
    match map.map_get(key_term) {
        Ok(val) => val
            .decode()
            .map_err(|_| rustler::Error::Term(Box::new(format!("{key} must be u32")))),
        Err(_) => Err(rustler::Error::Term(Box::new(format!(
            "missing key: {key}"
        )))),
    }
}

fn get_u64<'a>(env: Env<'a>, map: Term<'a>, key: &str) -> NifResult<u64> {
    let key_term = key.encode(env);
    match map.map_get(key_term) {
        Ok(val) => val
            .decode()
            .map_err(|_| rustler::Error::Term(Box::new(format!("{key} must be u64")))),
        Err(_) => Err(rustler::Error::Term(Box::new(format!(
            "missing key: {key}"
        )))),
    }
}

fn get_usize<'a>(env: Env<'a>, map: Term<'a>, key: &str) -> NifResult<usize> {
    let key_term = key.encode(env);
    match map.map_get(key_term) {
        Ok(val) => val
            .decode()
            .map_err(|_| rustler::Error::Term(Box::new(format!("{key} must be integer")))),
        Err(_) => Err(rustler::Error::Term(Box::new(format!(
            "missing key: {key}"
        )))),
    }
}

fn load(env: Env, _info: Term) -> bool {
    rustler::resource!(CoreResource, env);
    true
}

rustler::init!("Elixir.McpServerElixir.Infrastructure.CoreNif", load = load);
