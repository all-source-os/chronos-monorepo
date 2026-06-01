use crate::{
    domain::{
        entities::{SchemaEnforcement, Tenant, TenantQuotas, TenantUsage},
        repositories::TenantRepository,
        value_objects::{
            TenantId,
            system_stream::{SystemDomain, system_entity_id_value, tenant_events},
        },
    },
    error::{AllSourceError, Result},
    infrastructure::persistence::SystemMetadataStore,
};
use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::json;
use std::sync::Arc;

/// Event-sourced TenantRepository backed by SystemMetadataStore.
///
/// Tenant lifecycle events are stored in the `_system:tenant:*` stream and
/// materialized into an in-memory DashMap cache. On startup, the cache is
/// rebuilt by replaying all tenant events from the system store.
///
/// # Cache Consistency
///
/// Uses `DashMap<String, Tenant>` for concurrent O(1) lookups by tenant ID.
/// The write path is: WAL fsync → cache update (via `emit_event`), so no
/// acknowledged writes are lost even on crash. On startup, the cache is
/// rebuilt by replaying all tenant events from the system store.
///
/// # Event Types
///
/// - `_system.tenant.created` — Tenant creation with name, quotas, metadata
/// - `_system.tenant.updated` — Name, description, or metadata changes
/// - `_system.tenant.suspended` — Tenant deactivation
/// - `_system.tenant.reactivated` — Tenant reactivation
/// - `_system.tenant.deleted` — Tenant removal
/// - `_system.tenant.quota_updated` — Quota changes
/// - `_system.tenant.usage_updated` — Usage counter updates
pub struct EventSourcedTenantRepository {
    /// Durable storage for system events
    system_store: Arc<SystemMetadataStore>,

    /// In-memory cache of materialized tenant state
    cache: Arc<DashMap<String, Tenant>>,
}

impl EventSourcedTenantRepository {
    /// Create a new event-sourced tenant repository.
    ///
    /// Replays all existing tenant events from the system store to rebuild
    /// the in-memory cache.
    pub fn new(system_store: Arc<SystemMetadataStore>) -> Self {
        let cache = Arc::new(DashMap::new());
        let repo = Self {
            system_store,
            cache,
        };
        repo.rebuild_cache();
        repo
    }

    /// Rebuild the in-memory cache from all tenant events in the system store.
    fn rebuild_cache(&self) {
        let events = self.system_store.read_stream(SystemDomain::Tenant);
        for event in events {
            self.apply_event(&event);
        }
        tracing::info!(
            "Rebuilt tenant cache: {} tenants from system store",
            self.cache.len()
        );
    }

    /// Apply a single event to the in-memory cache.
    fn apply_event(&self, event: &crate::domain::entities::Event) {
        let event_type = event.event_type_str();
        let payload = event.payload();

        // Extract tenant_id from entity_id: _system:tenant:<tenant_id>
        let Some(tenant_id_str) = event.entity_id_str().strip_prefix("_system:tenant:") else {
            tracing::warn!(
                "Unexpected entity_id in tenant stream: {}",
                event.entity_id_str()
            );
            return;
        };

        match event_type {
            t if t == tenant_events::CREATED => {
                let name = payload["name"].as_str().unwrap_or("").to_string();
                let description = payload["description"]
                    .as_str()
                    .map(std::string::ToString::to_string);
                let quotas: TenantQuotas = payload
                    .get("quotas")
                    .and_then(|q| serde_json::from_value(q.clone()).ok())
                    .unwrap_or_default();
                let metadata = payload
                    .get("metadata")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                if let Ok(tid) = TenantId::new(tenant_id_str.to_string()) {
                    let is_demo = payload["is_demo"].as_bool().unwrap_or(false);
                    let tenant = Tenant::reconstruct(
                        tid,
                        name,
                        description,
                        quotas,
                        TenantUsage::new(),
                        event.timestamp(),
                        event.timestamp(),
                        true,
                        is_demo,
                        metadata,
                    );
                    self.cache.insert(tenant_id_str.to_string(), tenant);
                }
            }
            t if t == tenant_events::UPDATED => {
                if let Some(mut entry) = self.cache.get_mut(tenant_id_str) {
                    let tenant = entry.value_mut();
                    if let Some(name) = payload["name"].as_str() {
                        let _ = tenant.update_name(name.to_string());
                    }
                    if payload.get("description").is_some() {
                        let desc = payload["description"]
                            .as_str()
                            .map(std::string::ToString::to_string);
                        tenant.update_description(desc);
                    }
                    if let Some(is_demo) = payload["is_demo"].as_bool() {
                        tenant.set_is_demo(is_demo);
                    }
                    if let Some(metadata) = payload.get("metadata") {
                        tenant.update_metadata(metadata.clone());
                    }
                }
            }
            t if t == tenant_events::SUSPENDED => {
                if let Some(mut entry) = self.cache.get_mut(tenant_id_str) {
                    entry.value_mut().deactivate();
                }
            }
            t if t == tenant_events::REACTIVATED => {
                if let Some(mut entry) = self.cache.get_mut(tenant_id_str) {
                    entry.value_mut().activate();
                }
            }
            t if t == tenant_events::DELETED => {
                self.cache.remove(tenant_id_str);
            }
            t if t == tenant_events::QUOTA_UPDATED => {
                if let Some(mut entry) = self.cache.get_mut(tenant_id_str)
                    && let Ok(quotas) = serde_json::from_value::<TenantQuotas>(payload.clone())
                {
                    entry.value_mut().update_quotas(quotas);
                }
            }
            t if t == tenant_events::SCHEMA_ENFORCEMENT_UPDATED => {
                if let Some(mut entry) = self.cache.get_mut(tenant_id_str)
                    && let Ok(mode) = serde_json::from_value::<SchemaEnforcement>(payload.clone())
                {
                    entry.value_mut().set_schema_enforcement(mode);
                }
            }
            t if t == tenant_events::USAGE_UPDATED => {
                // During cache rebuild (replay), we just skip usage updates.
                // Usage is volatile and resets on restart. The event is preserved
                // in the WAL for audit purposes but doesn't need to be replayed.
            }
            _ => {
                tracing::debug!("Unknown tenant event type: {}", event_type);
            }
        }
    }

    /// Emit an event to the system store and apply it to the cache.
    fn emit_event(
        &self,
        event_type_str: &str,
        tenant_id_str: &str,
        payload: serde_json::Value,
    ) -> Result<()> {
        let entity_id = system_entity_id_value(SystemDomain::Tenant, tenant_id_str);
        let event =
            self.system_store
                .append_system_event(event_type_str, entity_id, payload, None)?;
        self.apply_event(&event);
        Ok(())
    }
}

#[async_trait]
impl TenantRepository for EventSourcedTenantRepository {
    async fn create(&self, id: TenantId, name: String, quotas: TenantQuotas) -> Result<Tenant> {
        let id_str = id.as_str().to_string();

        // Check for duplicates
        if self.cache.contains_key(&id_str) {
            return Err(AllSourceError::TenantAlreadyExists(id_str));
        }

        // Validate name early
        let tenant = Tenant::new(id.clone(), name.clone(), quotas.clone())?;

        // Emit creation event
        let payload = json!({
            "name": name,
            "quotas": serde_json::to_value(&quotas).unwrap_or_default(),
        });
        self.emit_event(tenant_events::CREATED, &id_str, payload)?;

        // Return the cached tenant (built by apply_event)
        self.cache
            .get(&id_str)
            .map(|entry| entry.value().clone())
            .ok_or_else(|| {
                AllSourceError::InternalError("Tenant not in cache after creation".to_string())
            })
    }

    async fn save(&self, tenant: &Tenant) -> Result<()> {
        let id_str = tenant.id().as_str().to_string();

        if self.cache.contains_key(&id_str) {
            // Update existing
            let payload = json!({
                "name": tenant.name(),
                "description": tenant.description(),
                "is_demo": tenant.is_demo(),
                "metadata": tenant.metadata(),
            });
            self.emit_event(tenant_events::UPDATED, &id_str, payload)?;

            // Also update quotas if changed
            if let Some(cached) = self.cache.get(&id_str)
                && cached.quotas() != tenant.quotas()
            {
                let quota_payload = serde_json::to_value(tenant.quotas()).unwrap_or_default();
                self.emit_event(tenant_events::QUOTA_UPDATED, &id_str, quota_payload)?;
            }

            // Sync active status
            let is_cached_active = self.cache.get(&id_str).map_or(true, |t| t.is_active());
            if tenant.is_active() != is_cached_active && tenant.is_active() {
                self.emit_event(tenant_events::REACTIVATED, &id_str, json!({}))?;
            } else if tenant.is_active() != is_cached_active {
                self.emit_event(tenant_events::SUSPENDED, &id_str, json!({}))?;
            }
        } else {
            // Create new
            let payload = json!({
                "name": tenant.name(),
                "description": tenant.description(),
                "quotas": serde_json::to_value(tenant.quotas()).unwrap_or_default(),
                "metadata": tenant.metadata(),
            });
            self.emit_event(tenant_events::CREATED, &id_str, payload)?;
        }

        Ok(())
    }

    async fn find_by_id(&self, id: &TenantId) -> Result<Option<Tenant>> {
        Ok(self
            .cache
            .get(id.as_str())
            .map(|entry| entry.value().clone()))
    }

    async fn find_by_name(&self, name: &str) -> Result<Option<Tenant>> {
        let name_lower = name.to_lowercase();
        Ok(self
            .cache
            .iter()
            .find(|entry| entry.value().name().to_lowercase() == name_lower)
            .map(|entry| entry.value().clone()))
    }

    async fn find_all(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let mut tenants: Vec<Tenant> = self.cache.iter().map(|e| e.value().clone()).collect();
        tenants.sort_by_key(|t| std::cmp::Reverse(t.created_at()));
        Ok(tenants.into_iter().skip(offset).take(limit).collect())
    }

    async fn find_active(&self, limit: usize, offset: usize) -> Result<Vec<Tenant>> {
        let mut tenants: Vec<Tenant> = self
            .cache
            .iter()
            .filter(|e| e.value().is_active())
            .map(|e| e.value().clone())
            .collect();
        tenants.sort_by_key(|t| std::cmp::Reverse(t.created_at()));
        Ok(tenants.into_iter().skip(offset).take(limit).collect())
    }

    async fn count(&self) -> Result<usize> {
        Ok(self.cache.len())
    }

    async fn count_active(&self) -> Result<usize> {
        Ok(self.cache.iter().filter(|e| e.value().is_active()).count())
    }

    async fn delete(&self, id: &TenantId) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        self.emit_event(tenant_events::DELETED, id_str, json!({}))?;
        Ok(true)
    }

    async fn update_quotas(&self, id: &TenantId, quotas: TenantQuotas) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        let payload = serde_json::to_value(&quotas).unwrap_or_default();
        self.emit_event(tenant_events::QUOTA_UPDATED, id_str, payload)?;
        // Apply directly to cache since apply_event handles deserialization
        if let Some(mut entry) = self.cache.get_mut(id_str) {
            entry.value_mut().update_quotas(quotas);
        }
        Ok(true)
    }

    async fn update_schema_enforcement(
        &self,
        id: &TenantId,
        mode: SchemaEnforcement,
    ) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        let payload = serde_json::to_value(mode).unwrap_or_default();
        self.emit_event(tenant_events::SCHEMA_ENFORCEMENT_UPDATED, id_str, payload)?;
        if let Some(mut entry) = self.cache.get_mut(id_str) {
            entry.value_mut().set_schema_enforcement(mode);
        }
        Ok(true)
    }

    async fn update_usage(&self, id: &TenantId, usage: TenantUsage) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        // Usage updates are high-frequency. We update the cache directly
        // and periodically flush to the system store. For now, just update cache.
        // The usage will be persisted via the save() method when needed.
        if let Some(mut entry) = self.cache.get_mut(id_str) {
            // We can't directly set usage, but we can use usage_mut()
            // to get a mutable reference and update fields
            let tenant_usage = entry.value_mut().usage_mut();
            // Since TenantUsage doesn't have a direct "set" method,
            // we need to reconstruct the tenant with the new usage
            drop(entry);

            // Get current tenant, reconstruct with new usage
            if let Some(current) = self.cache.get(id_str) {
                let tenant = current.value();
                let updated = Tenant::reconstruct(
                    tenant.id().clone(),
                    tenant.name().to_string(),
                    tenant.description().map(std::string::ToString::to_string),
                    tenant.quotas().clone(),
                    usage,
                    tenant.created_at(),
                    chrono::Utc::now(),
                    tenant.is_active(),
                    tenant.is_demo(),
                    tenant.metadata().clone(),
                );
                drop(current);
                self.cache.insert(id_str.to_string(), updated);
            }
        }
        Ok(true)
    }

    async fn activate(&self, id: &TenantId) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        self.emit_event(tenant_events::REACTIVATED, id_str, json!({}))?;
        Ok(true)
    }

    async fn deactivate(&self, id: &TenantId) -> Result<bool> {
        let id_str = id.as_str();
        if !self.cache.contains_key(id_str) {
            return Ok(false);
        }
        self.emit_event(tenant_events::SUSPENDED, id_str, json!({}))?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::persistence::SystemMetadataStore;
    use tempfile::TempDir;

    fn create_test_repo() -> (EventSourcedTenantRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let system_store =
            Arc::new(SystemMetadataStore::new(temp_dir.path().join("__system")).unwrap());
        let repo = EventSourcedTenantRepository::new(system_store);
        (repo, temp_dir)
    }

    #[tokio::test]
    async fn test_create_tenant() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme-corp".to_string()).unwrap();
        let tenant = repo
            .create(
                id.clone(),
                "ACME Corp".to_string(),
                TenantQuotas::standard(),
            )
            .await
            .unwrap();

        assert_eq!(tenant.name(), "ACME Corp");
        assert!(tenant.is_active());
    }

    #[tokio::test]
    async fn test_create_duplicate_tenant() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id.clone(), "ACME".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        let result = repo
            .create(id, "ACME Again".to_string(), TenantQuotas::standard())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_find_by_id() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id.clone(), "ACME".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "ACME");

        let not_found = TenantId::new("nonexistent".to_string()).unwrap();
        assert!(repo.find_by_id(&not_found).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn test_find_by_name() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id, "ACME Corp".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        let found = repo.find_by_name("acme corp").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "ACME Corp");
    }

    #[tokio::test]
    async fn test_find_all() {
        let (repo, _dir) = create_test_repo();

        for i in 0..5 {
            let id = TenantId::new(format!("tenant-{i}")).unwrap();
            repo.create(id, format!("Tenant {i}"), TenantQuotas::standard())
                .await
                .unwrap();
        }

        let all = repo.find_all(10, 0).await.unwrap();
        assert_eq!(all.len(), 5);

        let page = repo.find_all(2, 2).await.unwrap();
        assert_eq!(page.len(), 2);
    }

    #[tokio::test]
    async fn test_activate_deactivate() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id.clone(), "ACME".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        // Deactivate
        assert!(repo.deactivate(&id).await.unwrap());
        let tenant = repo.find_by_id(&id).await.unwrap().unwrap();
        assert!(!tenant.is_active());

        // Reactivate
        assert!(repo.activate(&id).await.unwrap());
        let tenant = repo.find_by_id(&id).await.unwrap().unwrap();
        assert!(tenant.is_active());
    }

    #[tokio::test]
    async fn test_delete_tenant() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id.clone(), "ACME".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        assert!(repo.delete(&id).await.unwrap());
        assert!(repo.find_by_id(&id).await.unwrap().is_none());
        assert_eq!(repo.count().await.unwrap(), 0);

        // Delete non-existent returns false
        assert!(!repo.delete(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_count() {
        let (repo, _dir) = create_test_repo();

        assert_eq!(repo.count().await.unwrap(), 0);
        assert_eq!(repo.count_active().await.unwrap(), 0);

        let id1 = TenantId::new("t1".to_string()).unwrap();
        let id2 = TenantId::new("t2".to_string()).unwrap();
        repo.create(id1.clone(), "T1".to_string(), TenantQuotas::standard())
            .await
            .unwrap();
        repo.create(id2.clone(), "T2".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 2);

        repo.deactivate(&id1).await.unwrap();
        assert_eq!(repo.count().await.unwrap(), 2);
        assert_eq!(repo.count_active().await.unwrap(), 1);
    }

    #[tokio::test]
    async fn test_update_quotas() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("acme".to_string()).unwrap();
        repo.create(id.clone(), "ACME".to_string(), TenantQuotas::free_tier())
            .await
            .unwrap();

        let new_quotas = TenantQuotas::professional();
        assert!(repo.update_quotas(&id, new_quotas.clone()).await.unwrap());

        let tenant = repo.find_by_id(&id).await.unwrap().unwrap();
        assert_eq!(tenant.quotas(), &new_quotas);

        // Non-existent tenant
        let bad_id = TenantId::new("nonexistent".to_string()).unwrap();
        assert!(
            !repo
                .update_quotas(&bad_id, TenantQuotas::standard())
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_cache_rebuild_on_restart() {
        let temp_dir = TempDir::new().unwrap();
        let system_dir = temp_dir.path().join("__system");

        // Create tenants
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedTenantRepository::new(system_store);

            repo.create(
                TenantId::new("acme".to_string()).unwrap(),
                "ACME Corp".to_string(),
                TenantQuotas::standard(),
            )
            .await
            .unwrap();

            repo.create(
                TenantId::new("beta".to_string()).unwrap(),
                "Beta Inc".to_string(),
                TenantQuotas::free_tier(),
            )
            .await
            .unwrap();

            // Deactivate one
            repo.deactivate(&TenantId::new("beta".to_string()).unwrap())
                .await
                .unwrap();
        }
        // Dropped — simulates restart

        // Rebuild from WAL
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedTenantRepository::new(system_store);

            assert_eq!(repo.count().await.unwrap(), 2);

            let acme = repo
                .find_by_id(&TenantId::new("acme".to_string()).unwrap())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(acme.name(), "ACME Corp");
            assert!(acme.is_active());

            let beta = repo
                .find_by_id(&TenantId::new("beta".to_string()).unwrap())
                .await
                .unwrap()
                .unwrap();
            assert_eq!(beta.name(), "Beta Inc");
            assert!(!beta.is_active());
        }
    }

    #[tokio::test]
    async fn test_schema_enforcement_survives_restart() {
        use crate::domain::entities::SchemaEnforcement;

        let temp_dir = TempDir::new().unwrap();
        let system_dir = temp_dir.path().join("__system");
        let tid = TenantId::new("acme".to_string()).unwrap();

        // Set Strict, then drop the repo (simulates restart).
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedTenantRepository::new(system_store);
            repo.create(tid.clone(), "ACME".to_string(), TenantQuotas::standard())
                .await
                .unwrap();
            // Default is Permissive.
            assert_eq!(
                repo.find_by_id(&tid).await.unwrap().unwrap().schema_enforcement(),
                SchemaEnforcement::Permissive
            );
            assert!(
                repo.update_schema_enforcement(&tid, SchemaEnforcement::Strict)
                    .await
                    .unwrap()
            );
            assert_eq!(
                repo.find_by_id(&tid).await.unwrap().unwrap().schema_enforcement(),
                SchemaEnforcement::Strict
            );
        }

        // Rebuild from WAL — the SCHEMA_ENFORCEMENT_UPDATED event must replay.
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedTenantRepository::new(system_store);
            assert_eq!(
                repo.find_by_id(&tid).await.unwrap().unwrap().schema_enforcement(),
                SchemaEnforcement::Strict,
                "schema enforcement mode must survive a restart"
            );
        }
    }

    #[tokio::test]
    async fn test_update_schema_enforcement_unknown_tenant_returns_false() {
        use crate::domain::entities::SchemaEnforcement;
        let (repo, _dir) = create_test_repo();
        let bad_id = TenantId::new("ghost".to_string()).unwrap();
        assert!(
            !repo
                .update_schema_enforcement(&bad_id, SchemaEnforcement::Strict)
                .await
                .unwrap()
        );
    }

    #[tokio::test]
    async fn test_find_active() {
        let (repo, _dir) = create_test_repo();

        let id1 = TenantId::new("active1".to_string()).unwrap();
        let id2 = TenantId::new("inactive1".to_string()).unwrap();
        let id3 = TenantId::new("active2".to_string()).unwrap();

        repo.create(id1, "Active 1".to_string(), TenantQuotas::standard())
            .await
            .unwrap();
        repo.create(
            id2.clone(),
            "Inactive 1".to_string(),
            TenantQuotas::standard(),
        )
        .await
        .unwrap();
        repo.create(id3, "Active 2".to_string(), TenantQuotas::standard())
            .await
            .unwrap();

        repo.deactivate(&id2).await.unwrap();

        let active = repo.find_active(10, 0).await.unwrap();
        assert_eq!(active.len(), 2);
        assert!(
            active
                .iter()
                .all(crate::domain::entities::tenant::Tenant::is_active)
        );
    }

    #[tokio::test]
    async fn test_save_new_tenant() {
        let (repo, _dir) = create_test_repo();

        let id = TenantId::new("new-tenant".to_string()).unwrap();
        let tenant = Tenant::new(
            id.clone(),
            "New Tenant".to_string(),
            TenantQuotas::standard(),
        )
        .unwrap();

        repo.save(&tenant).await.unwrap();

        let found = repo.find_by_id(&id).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().name(), "New Tenant");
    }
}
