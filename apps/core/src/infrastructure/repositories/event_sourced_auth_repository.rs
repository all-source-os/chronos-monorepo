use crate::{
    domain::value_objects::system_stream::{SystemDomain, auth_events, system_entity_id_value},
    error::Result,
    infrastructure::{
        persistence::SystemMetadataStore,
        security::auth::{ApiKey, Role},
    },
};
use chrono::{DateTime, Utc};
use dashmap::DashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Event-sourced authentication repository backed by SystemMetadataStore.
///
/// Stores API key provisioning and revocation as durable system events.
/// On startup, replays all `_system.auth.*` events to rebuild the key cache.
///
/// # Event Types
///
/// - `_system.auth.key_provisioned` — API key created (stores key hash, not raw key)
/// - `_system.auth.key_revoked` — API key deactivated
pub struct EventSourcedAuthRepository {
    system_store: Arc<SystemMetadataStore>,
    /// Cache of API keys rebuilt from events, keyed by key ID.
    cache: Arc<DashMap<Uuid, ApiKey>>,
}

impl EventSourcedAuthRepository {
    /// Create a new event-sourced auth repository.
    ///
    /// Replays all existing auth events from the system store.
    pub fn new(system_store: Arc<SystemMetadataStore>) -> Self {
        let cache = Arc::new(DashMap::new());
        let repo = Self {
            system_store,
            cache,
        };
        repo.rebuild_cache();
        repo
    }

    /// Rebuild the in-memory cache from system store events.
    fn rebuild_cache(&self) {
        let events = self.system_store.read_stream(SystemDomain::Auth);
        for event in events {
            let event_type = event.event_type_str();
            let payload = event.payload();

            let Some(key_id_str) = event.entity_id_str().strip_prefix("_system:auth:") else {
                continue;
            };

            let Ok(key_id) = Uuid::parse_str(key_id_str) else {
                continue;
            };

            match event_type {
                t if t == auth_events::KEY_PROVISIONED => {
                    let api_key = ApiKey {
                        id: key_id,
                        name: payload["name"].as_str().unwrap_or("").to_string(),
                        tenant_id: payload["tenant_id"]
                            .as_str()
                            .unwrap_or("default")
                            .to_string(),
                        role: serde_json::from_value(
                            payload
                                .get("role")
                                .cloned()
                                .unwrap_or(serde_json::json!("admin")),
                        )
                        .unwrap_or(Role::Admin),
                        key_hash: payload["key_hash"].as_str().unwrap_or("").to_string(),
                        created_at: payload["created_at"]
                            .as_str()
                            .and_then(|s| s.parse::<DateTime<Utc>>().ok())
                            .unwrap_or_else(Utc::now),
                        expires_at: payload["expires_at"]
                            .as_str()
                            .and_then(|s| s.parse::<DateTime<Utc>>().ok()),
                        active: true,
                        last_used: None,
                    };
                    self.cache.insert(key_id, api_key);
                }
                t if t == auth_events::KEY_REVOKED => {
                    if let Some(mut key) = self.cache.get_mut(&key_id) {
                        key.active = false;
                    }
                }
                _ => {}
            }
        }
        let active = self.cache.iter().filter(|e| e.value().active).count();
        tracing::info!(
            "Rebuilt auth cache: {} API keys ({} active) from system store",
            self.cache.len(),
            active
        );
    }

    /// Persist an API key to the system store.
    ///
    /// The raw key is NOT stored — only the hash. The caller is responsible
    /// for returning the raw key to the user at creation time.
    pub fn persist_api_key(&self, api_key: &ApiKey) -> Result<()> {
        let entity_id = system_entity_id_value(SystemDomain::Auth, &api_key.id.to_string());
        let payload = serde_json::json!({
            "name": api_key.name,
            "tenant_id": api_key.tenant_id,
            "role": api_key.role,
            "key_hash": api_key.key_hash,
            "created_at": api_key.created_at.to_rfc3339(),
            "expires_at": api_key.expires_at.map(|t| t.to_rfc3339()),
        });

        self.system_store.append_system_event(
            auth_events::KEY_PROVISIONED,
            entity_id,
            payload,
            None,
        )?;

        self.cache.insert(api_key.id, api_key.clone());
        Ok(())
    }

    /// Persist a key revocation event.
    pub fn persist_key_revocation(&self, key_id: &Uuid) -> Result<()> {
        let entity_id = system_entity_id_value(SystemDomain::Auth, &key_id.to_string());
        let payload = serde_json::json!({
            "revoked_at": Utc::now().to_rfc3339(),
        });

        self.system_store.append_system_event(
            auth_events::KEY_REVOKED,
            entity_id,
            payload,
            None,
        )?;

        if let Some(mut key) = self.cache.get_mut(key_id) {
            key.active = false;
        }
        Ok(())
    }

    /// Get all cached API keys (for loading into AuthManager).
    pub fn all_keys(&self) -> Vec<ApiKey> {
        self.cache.iter().map(|e| e.value().clone()).collect()
    }

    /// Count of API keys (including revoked).
    pub fn count(&self) -> usize {
        self.cache.len()
    }

    /// Count of active API keys.
    pub fn active_count(&self) -> usize {
        self.cache.iter().filter(|e| e.value().active).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::persistence::SystemMetadataStore;
    use tempfile::TempDir;

    fn create_test_repo() -> (EventSourcedAuthRepository, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let system_store =
            Arc::new(SystemMetadataStore::new(temp_dir.path().join("__system")).unwrap());
        let repo = EventSourcedAuthRepository::new(system_store);
        (repo, temp_dir)
    }

    fn make_test_key(name: &str, tenant: &str) -> ApiKey {
        ApiKey {
            id: Uuid::new_v4(),
            name: name.to_string(),
            tenant_id: tenant.to_string(),
            role: Role::Admin,
            key_hash: "testhash123".to_string(),
            created_at: Utc::now(),
            expires_at: None,
            active: true,
            last_used: None,
        }
    }

    #[test]
    fn test_persist_and_recover() {
        let temp_dir = TempDir::new().unwrap();
        let system_dir = temp_dir.path().join("__system");

        let key_id;
        // Write
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuthRepository::new(system_store);

            let key = make_test_key("bootstrap", "default");
            key_id = key.id;
            repo.persist_api_key(&key).unwrap();
            assert_eq!(repo.count(), 1);
        }

        // Recover
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuthRepository::new(system_store);

            assert_eq!(repo.count(), 1);
            assert_eq!(repo.active_count(), 1);
            let keys = repo.all_keys();
            assert_eq!(keys[0].id, key_id);
            assert_eq!(keys[0].name, "bootstrap");
            assert_eq!(keys[0].tenant_id, "default");
        }
    }

    #[test]
    fn test_revoke_and_recover() {
        let temp_dir = TempDir::new().unwrap();
        let system_dir = temp_dir.path().join("__system");

        let key_id;
        // Write + revoke
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuthRepository::new(system_store);

            let key = make_test_key("temp-key", "tenant1");
            key_id = key.id;
            repo.persist_api_key(&key).unwrap();
            repo.persist_key_revocation(&key_id).unwrap();
            assert_eq!(repo.active_count(), 0);
        }

        // Recover — revocation should survive
        {
            let system_store = Arc::new(SystemMetadataStore::new(&system_dir).unwrap());
            let repo = EventSourcedAuthRepository::new(system_store);

            assert_eq!(repo.count(), 1);
            assert_eq!(repo.active_count(), 0);
            let keys = repo.all_keys();
            assert!(!keys[0].active);
        }
    }

    #[test]
    fn test_empty_repo() {
        let (repo, _dir) = create_test_repo();
        assert_eq!(repo.count(), 0);
        assert_eq!(repo.active_count(), 0);
        assert!(repo.all_keys().is_empty());
    }

    #[test]
    fn test_multiple_keys() {
        let (repo, _dir) = create_test_repo();

        repo.persist_api_key(&make_test_key("key1", "tenant1"))
            .unwrap();
        repo.persist_api_key(&make_test_key("key2", "tenant2"))
            .unwrap();
        repo.persist_api_key(&make_test_key("key3", "tenant1"))
            .unwrap();

        assert_eq!(repo.count(), 3);
        assert_eq!(repo.active_count(), 3);
    }
}
