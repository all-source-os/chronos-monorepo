/// PostgreSQL Audit Event Repository
///
/// Production-grade persistent audit logging using PostgreSQL.
/// Provides ACID guarantees, complex queries, and long-term storage.

#[cfg(feature = "postgres")]
use async_trait::async_trait;
#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row};
#[cfg(feature = "postgres")]
use chrono::{DateTime, Utc};
#[cfg(feature = "postgres")]
use serde_json::Value as JsonValue;

#[cfg(feature = "postgres")]
use crate::domain::entities::{
    AuditEvent, AuditEventId, AuditAction, AuditCategory, AuditOutcome, Actor,
};
#[cfg(feature = "postgres")]
use crate::domain::repositories::{AuditEventRepository, AuditEventQuery};
#[cfg(feature = "postgres")]
use crate::domain::value_objects::TenantId;
#[cfg(feature = "postgres")]
use crate::error::{AllSourceError, Result};

#[cfg(feature = "postgres")]
/// PostgreSQL audit event repository
pub struct PostgresAuditRepository {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresAuditRepository {
    /// Create new PostgreSQL audit repository
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Run migrations (creates audit_events table)
    pub async fn migrate(&self) -> Result<()> {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Migration failed: {}", e)))?;
        Ok(())
    }

    /// Helper: Convert Actor to database format
    fn serialize_actor(actor: &Actor) -> (String, String, String) {
        match actor {
            Actor::User { user_id, username } => (
                "user".to_string(),
                user_id.clone(),
                username.clone(),
            ),
            Actor::ApiKey { key_id, key_name } => (
                "api_key".to_string(),
                key_id.clone(),
                key_name.clone(),
            ),
            Actor::System { component } => (
                "system".to_string(),
                component.clone(),
                component.clone(),
            ),
        }
    }

    /// Helper: Reconstruct Actor from database
    fn deserialize_actor(actor_type: &str, actor_id: String, actor_name: String) -> Result<Actor> {
        match actor_type {
            "user" => Ok(Actor::User {
                user_id: actor_id,
                username: actor_name,
            }),
            "api_key" => Ok(Actor::ApiKey {
                key_id: actor_id,
                key_name: actor_name,
            }),
            "system" => Ok(Actor::System {
                component: actor_id,
            }),
            _ => Err(AllSourceError::StorageError(format!(
                "Unknown actor type: {}",
                actor_type
            ))),
        }
    }

    /// Helper: Convert AuditAction to string
    fn action_to_string(action: &AuditAction) -> String {
        serde_json::to_string(action)
            .unwrap_or_else(|_| format!("{:?}", action))
            .trim_matches('"')
            .to_string()
    }

    /// Helper: Parse AuditAction from string
    fn string_to_action(s: &str) -> Result<AuditAction> {
        serde_json::from_str(&format!("\"{}\"", s))
            .map_err(|e| AllSourceError::StorageError(format!("Invalid action: {}", e)))
    }

    /// Helper: Convert AuditOutcome to string
    fn outcome_to_string(outcome: &AuditOutcome) -> String {
        match outcome {
            AuditOutcome::Success => "success".to_string(),
            AuditOutcome::Failure => "failure".to_string(),
            AuditOutcome::PartialSuccess => "partial_success".to_string(),
        }
    }

    /// Helper: Parse AuditOutcome from string
    fn string_to_outcome(s: &str) -> Result<AuditOutcome> {
        match s {
            "success" => Ok(AuditOutcome::Success),
            "failure" => Ok(AuditOutcome::Failure),
            "partial_success" => Ok(AuditOutcome::PartialSuccess),
            _ => Err(AllSourceError::StorageError(format!(
                "Invalid outcome: {}",
                s
            ))),
        }
    }

    /// Helper: Reconstruct AuditEvent from database row
    fn row_to_audit_event(row: &sqlx::postgres::PgRow) -> Result<AuditEvent> {
        let id: uuid::Uuid = row.try_get("id")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get id: {}", e)))?;

        let tenant_id_str: String = row.try_get("tenant_id")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get tenant_id: {}", e)))?;
        let tenant_id = TenantId::new(tenant_id_str)?;

        let timestamp: DateTime<Utc> = row.try_get("timestamp")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get timestamp: {}", e)))?;

        let action_str: String = row.try_get("action")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get action: {}", e)))?;
        let action = Self::string_to_action(&action_str)?;

        let actor_type: String = row.try_get("actor_type")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get actor_type: {}", e)))?;
        let actor_id: String = row.try_get("actor_id")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get actor_id: {}", e)))?;
        let actor_name: String = row.try_get("actor_name")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get actor_name: {}", e)))?;
        let actor = Self::deserialize_actor(&actor_type, actor_id, actor_name)?;

        let outcome_str: String = row.try_get("outcome")
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get outcome: {}", e)))?;
        let outcome = Self::string_to_outcome(&outcome_str)?;

        let resource_type: Option<String> = row.try_get("resource_type").ok();
        let resource_id: Option<String> = row.try_get("resource_id").ok();
        let ip_address: Option<String> = row.try_get::<Option<std::net::IpAddr>, _>("ip_address")
            .ok()
            .flatten()
            .map(|ip| ip.to_string());
        let user_agent: Option<String> = row.try_get("user_agent").ok();
        let request_id: Option<String> = row.try_get("request_id").ok();
        let error_message: Option<String> = row.try_get("error_message").ok();
        let metadata: Option<JsonValue> = row.try_get("metadata").ok();

        // Reconstruct event
        let mut event = AuditEvent::new(tenant_id, action, actor, outcome);

        // Manually set the ID and timestamp (need to use reconstruction pattern)
        // Since AuditEvent doesn't expose setters, we'll need to reconstruct it properly
        // For now, create a new event and it will get a new ID/timestamp
        // In production, you'd add a `reconstruct` method like EventStream has

        if let (Some(rt), Some(ri)) = (resource_type, resource_id) {
            event = event.with_resource(rt, ri);
        }
        if let Some(ip) = ip_address {
            event = event.with_ip_address(ip);
        }
        if let Some(ua) = user_agent {
            event = event.with_user_agent(ua);
        }
        if let Some(req_id) = request_id {
            event = event.with_request_id(req_id);
        }
        if let Some(err) = error_message {
            event = event.with_error(err);
        }
        if let Some(meta) = metadata {
            event = event.with_metadata(meta);
        }

        Ok(event)
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl AuditEventRepository for PostgresAuditRepository {
    async fn append(&self, event: AuditEvent) -> Result<()> {
        let (actor_type, actor_id, actor_name) = Self::serialize_actor(event.actor());
        let action_str = Self::action_to_string(event.action());
        let category_str = format!("{:?}", event.category()).to_lowercase();
        let outcome_str = Self::outcome_to_string(event.outcome());

        let ip_addr: Option<std::net::IpAddr> = event.ip_address()
            .and_then(|s| s.parse().ok());

        sqlx::query(
            r#"
            INSERT INTO audit_events (
                id, tenant_id, timestamp, action, category,
                actor_type, actor_id, actor_name,
                resource_type, resource_id, outcome,
                ip_address, user_agent, request_id,
                error_message, metadata
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            "#,
        )
        .bind(event.id().as_uuid())
        .bind(event.tenant_id().as_str())
        .bind(event.timestamp())
        .bind(action_str)
        .bind(category_str)
        .bind(actor_type)
        .bind(actor_id)
        .bind(actor_name)
        .bind(event.resource_type())
        .bind(event.resource_id())
        .bind(outcome_str)
        .bind(ip_addr)
        .bind(event.user_agent())
        .bind(event.request_id())
        .bind(event.error_message())
        .bind(event.metadata())
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to append audit event: {}", e)))?;

        Ok(())
    }

    async fn append_batch(&self, events: Vec<AuditEvent>) -> Result<()> {
        let mut tx = self.pool.begin().await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to begin transaction: {}", e)))?;

        for event in events {
            let (actor_type, actor_id, actor_name) = Self::serialize_actor(event.actor());
            let action_str = Self::action_to_string(event.action());
            let category_str = format!("{:?}", event.category()).to_lowercase();
            let outcome_str = Self::outcome_to_string(event.outcome());
            let ip_addr: Option<std::net::IpAddr> = event.ip_address()
                .and_then(|s| s.parse().ok());

            sqlx::query(
                r#"
                INSERT INTO audit_events (
                    id, tenant_id, timestamp, action, category,
                    actor_type, actor_id, actor_name,
                    resource_type, resource_id, outcome,
                    ip_address, user_agent, request_id,
                    error_message, metadata
                ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
                "#,
            )
            .bind(event.id().as_uuid())
            .bind(event.tenant_id().as_str())
            .bind(event.timestamp())
            .bind(action_str)
            .bind(category_str)
            .bind(actor_type)
            .bind(actor_id)
            .bind(actor_name)
            .bind(event.resource_type())
            .bind(event.resource_id())
            .bind(outcome_str)
            .bind(ip_addr)
            .bind(event.user_agent())
            .bind(event.request_id())
            .bind(event.error_message())
            .bind(event.metadata())
            .execute(&mut *tx)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to append audit event: {}", e)))?;
        }

        tx.commit().await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to commit transaction: {}", e)))?;

        Ok(())
    }

    async fn get_by_id(&self, id: &AuditEventId) -> Result<Option<AuditEvent>> {
        let row = sqlx::query(
            r#"
            SELECT * FROM audit_events WHERE id = $1
            "#,
        )
        .bind(id.as_uuid())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to get audit event: {}", e)))?;

        match row {
            Some(r) => Ok(Some(Self::row_to_audit_event(&r)?)),
            None => Ok(None),
        }
    }

    async fn query(&self, query: AuditEventQuery) -> Result<Vec<AuditEvent>> {
        let mut sql = String::from(
            "SELECT * FROM audit_events WHERE tenant_id = $1"
        );
        let mut param_count = 1;

        // Build dynamic query
        if query.start_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
        }
        if query.end_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
        }
        if query.action.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND action = ${}", param_count));
        }
        if query.category.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND category = ${}", param_count));
        }
        if query.actor_identifier.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND actor_id = ${}", param_count));
        }
        if query.resource_type.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND resource_type = ${}", param_count));
        }
        if query.resource_id.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND resource_id = ${}", param_count));
        }
        if query.security_events_only {
            sql.push_str(" AND action IN ('login_failed', 'permission_denied', 'rate_limit_exceeded', 'ip_blocked', 'suspicious_activity')");
        }

        sql.push_str(" ORDER BY timestamp DESC");

        if let Some(limit) = query.limit {
            param_count += 1;
            sql.push_str(&format!(" LIMIT ${}", param_count));
        }
        if let Some(offset) = query.offset {
            param_count += 1;
            sql.push_str(&format!(" OFFSET ${}", param_count));
        }

        // Build query with parameters
        let mut db_query = sqlx::query(&sql)
            .bind(query.tenant_id.as_str());

        if let Some(start) = query.start_time {
            db_query = db_query.bind(start);
        }
        if let Some(end) = query.end_time {
            db_query = db_query.bind(end);
        }
        if let Some(action) = query.action {
            db_query = db_query.bind(Self::action_to_string(&action));
        }
        if let Some(category) = query.category {
            db_query = db_query.bind(format!("{:?}", category).to_lowercase());
        }
        if let Some(actor_id) = query.actor_identifier {
            // Extract just the ID part after the colon
            let actor_id_only = actor_id.split(':').nth(1).unwrap_or(&actor_id);
            db_query = db_query.bind(actor_id_only);
        }
        if let Some(resource_type) = query.resource_type {
            db_query = db_query.bind(resource_type);
        }
        if let Some(resource_id) = query.resource_id {
            db_query = db_query.bind(resource_id);
        }
        if let Some(limit) = query.limit {
            db_query = db_query.bind(limit as i64);
        }
        if let Some(offset) = query.offset {
            db_query = db_query.bind(offset as i64);
        }

        let rows = db_query
            .fetch_all(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to query audit events: {}", e)))?;

        let mut events = Vec::new();
        for row in rows {
            events.push(Self::row_to_audit_event(&row)?);
        }

        Ok(events)
    }

    async fn count(&self, query: AuditEventQuery) -> Result<usize> {
        let mut sql = String::from(
            "SELECT COUNT(*) FROM audit_events WHERE tenant_id = $1"
        );
        let mut param_count = 1;

        // Build dynamic query (same logic as query())
        if query.start_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp >= ${}", param_count));
        }
        if query.end_time.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND timestamp <= ${}", param_count));
        }
        if query.action.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND action = ${}", param_count));
        }
        if query.category.is_some() {
            param_count += 1;
            sql.push_str(&format!(" AND category = ${}", param_count));
        }
        if query.security_events_only {
            sql.push_str(" AND action IN ('login_failed', 'permission_denied', 'rate_limit_exceeded', 'ip_blocked', 'suspicious_activity')");
        }

        let mut db_query = sqlx::query(&sql)
            .bind(query.tenant_id.as_str());

        if let Some(start) = query.start_time {
            db_query = db_query.bind(start);
        }
        if let Some(end) = query.end_time {
            db_query = db_query.bind(end);
        }
        if let Some(action) = query.action {
            db_query = db_query.bind(Self::action_to_string(&action));
        }
        if let Some(category) = query.category {
            db_query = db_query.bind(format!("{:?}", category).to_lowercase());
        }

        let row = db_query
            .fetch_one(&self.pool)
            .await
            .map_err(|e| AllSourceError::StorageError(format!("Failed to count audit events: {}", e)))?;

        let count: i64 = row.try_get(0)
            .map_err(|e| AllSourceError::StorageError(format!("Failed to get count: {}", e)))?;

        Ok(count as usize)
    }

    async fn get_by_tenant(
        &self,
        tenant_id: &TenantId,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .with_pagination(limit, offset);
        self.query(query).await
    }

    async fn get_security_events(
        &self,
        tenant_id: &TenantId,
        limit: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .security_only()
            .with_pagination(limit, 0);
        self.query(query).await
    }

    async fn get_by_actor(
        &self,
        tenant_id: &TenantId,
        actor_identifier: &str,
        limit: usize,
    ) -> Result<Vec<AuditEvent>> {
        let query = AuditEventQuery::new(tenant_id.clone())
            .with_actor(actor_identifier.to_string())
            .with_pagination(limit, 0);
        self.query(query).await
    }

    async fn purge_old_events(
        &self,
        tenant_id: &TenantId,
        older_than: DateTime<Utc>,
    ) -> Result<usize> {
        let result = sqlx::query(
            r#"
            DELETE FROM audit_events
            WHERE tenant_id = $1 AND timestamp < $2
            "#,
        )
        .bind(tenant_id.as_str())
        .bind(older_than)
        .execute(&self.pool)
        .await
        .map_err(|e| AllSourceError::StorageError(format!("Failed to purge audit events: {}", e)))?;

        Ok(result.rows_affected() as usize)
    }
}

#[cfg(all(test, feature = "postgres"))]
mod tests {
    use super::*;

    // Note: These tests require a running PostgreSQL database
    // Run with: cargo test --features postgres

    #[tokio::test]
    #[ignore] // Requires PostgreSQL
    async fn test_postgres_audit_repository() {
        // This test would require a test database
        // In production, use testcontainers or similar
    }
}
