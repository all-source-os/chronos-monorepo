/// ML-Based Anomaly Detection System
///
/// Detects suspicious patterns in audit logs using statistical analysis
/// and machine learning techniques to identify:
/// - Unusual access patterns
/// - Brute force attacks
/// - Privilege escalation attempts
/// - Data exfiltration patterns
/// - Account compromise indicators

use crate::domain::entities::{AuditEvent, AuditAction, AuditOutcome};
use crate::domain::value_objects::TenantId;
use crate::error::{AllSourceError, Result};
use chrono::{DateTime, Duration, Timelike, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

/// Anomaly detection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyDetectionConfig {
    /// Enable anomaly detection
    pub enabled: bool,

    /// Sensitivity level (0.0 = very lenient, 1.0 = very strict)
    pub sensitivity: f64,

    /// Minimum events required for baseline
    pub min_baseline_events: usize,

    /// Time window for analysis (hours)
    pub analysis_window_hours: i64,

    /// Enable specific detectors
    pub enable_brute_force_detection: bool,
    pub enable_unusual_access_detection: bool,
    pub enable_privilege_escalation_detection: bool,
    pub enable_data_exfiltration_detection: bool,
    pub enable_velocity_detection: bool,
}

impl Default for AnomalyDetectionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sensitivity: 0.7,
            min_baseline_events: 100,
            analysis_window_hours: 24,
            enable_brute_force_detection: true,
            enable_unusual_access_detection: true,
            enable_privilege_escalation_detection: true,
            enable_data_exfiltration_detection: true,
            enable_velocity_detection: true,
        }
    }
}

/// Anomaly detection result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    /// Is this event anomalous?
    pub is_anomalous: bool,

    /// Anomaly score (0.0 = normal, 1.0 = highly anomalous)
    pub score: f64,

    /// Type of anomaly detected
    pub anomaly_type: Option<AnomalyType>,

    /// Detailed reason
    pub reason: String,

    /// Recommended action
    pub recommended_action: RecommendedAction,

    /// Contributing factors
    pub factors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AnomalyType {
    BruteForceAttack,
    UnusualAccessPattern,
    PrivilegeEscalation,
    DataExfiltration,
    VelocityAnomaly,
    AccountCompromise,
    SuspiciousActivity,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum RecommendedAction {
    Monitor,          // Continue monitoring
    Alert,            // Send alert to security team
    Block,            // Block the action
    RequireMFA,       // Require additional authentication
    RevokeAccess,     // Immediately revoke access
}

/// User behavior profile for baseline comparison
#[derive(Debug, Clone, Serialize, Deserialize)]
struct UserProfile {
    user_id: String,
    tenant_id: String,

    // Activity patterns
    typical_hours: Vec<u32>,          // Hours of day when user is typically active
    typical_actions: HashMap<AuditAction, usize>,  // Action frequency
    typical_locations: Vec<String>,   // IP addresses or locations

    // Statistical baselines
    avg_actions_per_hour: f64,
    avg_actions_per_day: f64,
    max_actions_per_hour: usize,

    // Failure rates
    avg_failure_rate: f64,

    // Last updated
    last_updated: DateTime<Utc>,
    event_count: usize,
}

impl UserProfile {
    fn new(user_id: String, tenant_id: String) -> Self {
        Self {
            user_id,
            tenant_id,
            typical_hours: Vec::new(),
            typical_actions: HashMap::new(),
            typical_locations: Vec::new(),
            avg_actions_per_hour: 0.0,
            avg_actions_per_day: 0.0,
            max_actions_per_hour: 0,
            avg_failure_rate: 0.0,
            last_updated: Utc::now(),
            event_count: 0,
        }
    }
}

/// Tenant behavior profile for organizational patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TenantProfile {
    tenant_id: String,

    // Access patterns
    typical_daily_events: f64,
    typical_hourly_events: f64,
    peak_hours: Vec<u32>,

    // User activity
    active_users_per_day: f64,

    // Security metrics
    avg_failure_rate: f64,
    suspicious_event_rate: f64,

    last_updated: DateTime<Utc>,
    event_count: usize,
}

/// ML-based anomaly detector
pub struct AnomalyDetector {
    config: Arc<RwLock<AnomalyDetectionConfig>>,

    // Behavior profiles
    user_profiles: Arc<RwLock<HashMap<String, UserProfile>>>,
    tenant_profiles: Arc<RwLock<HashMap<String, TenantProfile>>>,

    // Recent events for pattern analysis
    recent_events: Arc<RwLock<Vec<AuditEvent>>>,
}

impl AnomalyDetector {
    /// Create new anomaly detector
    pub fn new(config: AnomalyDetectionConfig) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            user_profiles: Arc::new(RwLock::new(HashMap::new())),
            tenant_profiles: Arc::new(RwLock::new(HashMap::new())),
            recent_events: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Analyze an audit event for anomalies
    pub fn analyze_event(&self, event: &AuditEvent) -> Result<AnomalyResult> {
        let config = self.config.read();

        if !config.enabled {
            return Ok(AnomalyResult {
                is_anomalous: false,
                score: 0.0,
                anomaly_type: None,
                reason: "Anomaly detection disabled".to_string(),
                recommended_action: RecommendedAction::Monitor,
                factors: vec![],
            });
        }

        let mut anomaly_scores: Vec<(AnomalyType, f64, Vec<String>)> = Vec::new();

        // Get user ID from actor
        let user_id = match event.actor() {
            crate::domain::entities::Actor::User { user_id, .. } => user_id.clone(),
            crate::domain::entities::Actor::System { .. } => {
                // System actions are typically not anomalous
                return Ok(AnomalyResult {
                    is_anomalous: false,
                    score: 0.0,
                    anomaly_type: None,
                    reason: "System actor".to_string(),
                    recommended_action: RecommendedAction::Monitor,
                    factors: vec![],
                });
            }
            crate::domain::entities::Actor::ApiKey { key_id, key_name: _ } => key_id.clone(),
        };

        // Check for brute force attacks
        if config.enable_brute_force_detection {
            if let Some((score, factors)) = self.detect_brute_force(&user_id, event)? {
                anomaly_scores.push((AnomalyType::BruteForceAttack, score, factors));
            }
        }

        // Check for unusual access patterns
        if config.enable_unusual_access_detection {
            if let Some((score, factors)) = self.detect_unusual_access(&user_id, event)? {
                anomaly_scores.push((AnomalyType::UnusualAccessPattern, score, factors));
            }
        }

        // Check for privilege escalation
        if config.enable_privilege_escalation_detection {
            if let Some((score, factors)) = self.detect_privilege_escalation(&user_id, event)? {
                anomaly_scores.push((AnomalyType::PrivilegeEscalation, score, factors));
            }
        }

        // Check for data exfiltration
        if config.enable_data_exfiltration_detection {
            if let Some((score, factors)) = self.detect_data_exfiltration(&user_id, event)? {
                anomaly_scores.push((AnomalyType::DataExfiltration, score, factors));
            }
        }

        // Check for velocity anomalies
        if config.enable_velocity_detection {
            if let Some((score, factors)) = self.detect_velocity_anomaly(&user_id, event)? {
                anomaly_scores.push((AnomalyType::VelocityAnomaly, score, factors));
            }
        }

        // Store event for future analysis
        self.add_recent_event(event.clone());

        // Calculate overall anomaly score (max of all detectors)
        let (max_anomaly_type, max_score, all_factors) = if anomaly_scores.is_empty() {
            (None, 0.0, vec![])
        } else {
            let max_entry = anomaly_scores.iter().max_by(|a, b| a.1.partial_cmp(&b.1).unwrap()).unwrap();
            let all_factors: Vec<String> = anomaly_scores.iter().flat_map(|(_,  _, f)| f.clone()).collect();
            (Some(max_entry.0.clone()), max_entry.1, all_factors)
        };

        let is_anomalous = max_score >= config.sensitivity;

        let recommended_action = if max_score >= 0.9 {
            RecommendedAction::RevokeAccess
        } else if max_score >= 0.8 {
            RecommendedAction::Block
        } else if max_score >= 0.7 {
            RecommendedAction::RequireMFA
        } else if max_score >= 0.5 {
            RecommendedAction::Alert
        } else {
            RecommendedAction::Monitor
        };

        let reason = if is_anomalous {
            format!("Anomalous {:?} detected with score {:.2}", max_anomaly_type.as_ref().unwrap(), max_score)
        } else {
            "Normal behavior".to_string()
        };

        Ok(AnomalyResult {
            is_anomalous,
            score: max_score,
            anomaly_type: max_anomaly_type,
            reason,
            recommended_action,
            factors: all_factors,
        })
    }

    /// Update user profile with new event
    pub fn update_profile(&self, event: &AuditEvent) -> Result<()> {
        let user_id = match event.actor() {
            crate::domain::entities::Actor::User { user_id, .. } => user_id.clone(),
            crate::domain::entities::Actor::ApiKey { key_id, key_name: _ } => key_id.clone(),
            _ => return Ok(()),
        };

        let mut profiles = self.user_profiles.write();
        let profile = profiles.entry(format!("{}-{}", event.tenant_id().as_str(), user_id))
            .or_insert_with(|| UserProfile::new(user_id.clone(), event.tenant_id().as_str().to_string()));

        // Update activity patterns
        let hour = event.timestamp().hour();
        if !profile.typical_hours.contains(&hour) {
            profile.typical_hours.push(hour);
        }

        // Update action frequency
        *profile.typical_actions.entry(event.action().clone()).or_insert(0) += 1;

        // Update statistics
        profile.event_count += 1;
        profile.last_updated = Utc::now();

        // Recalculate averages (simple moving average)
        let events_in_window = profile.event_count.min(1000);
        profile.avg_actions_per_hour = events_in_window as f64 / 24.0;

        Ok(())
    }

    // === Detection Methods ===

    fn detect_brute_force(&self, user_id: &str, event: &AuditEvent) -> Result<Option<(f64, Vec<String>)>> {
        // Detect multiple failed login attempts
        if event.action() != &AuditAction::Login {
            return Ok(None);
        }

        let recent = self.recent_events.read();
        let mut recent_failures = recent.iter()
            .filter(|e| {
                if let crate::domain::entities::Actor::User { user_id: uid, .. } = e.actor() {
                    uid == user_id && e.action() == &AuditAction::Login
                        && e.outcome() == &AuditOutcome::Failure
                        && (Utc::now() - e.timestamp()) < Duration::minutes(15)
                } else {
                    false
                }
            })
            .count();

        // Include current event if it's also a failure
        if event.outcome() == &AuditOutcome::Failure {
            recent_failures += 1;
        }

        if recent_failures >= 5 {
            let score = (recent_failures as f64 / 10.0).min(1.0);
            let factors = vec![
                format!("{} failed login attempts in 15 minutes", recent_failures),
            ];
            return Ok(Some((score, factors)));
        }

        Ok(None)
    }

    fn detect_unusual_access(&self, user_id: &str, event: &AuditEvent) -> Result<Option<(f64, Vec<String>)>> {
        let profiles = self.user_profiles.read();
        let profile_key = format!("{}-{}", event.tenant_id().as_str(), user_id);

        if let Some(profile) = profiles.get(&profile_key) {
            if profile.event_count < self.config.read().min_baseline_events {
                return Ok(None); // Not enough data for baseline
            }

            let mut factors = Vec::new();
            let mut anomaly_indicators = 0;

            // Check if access is outside typical hours
            let hour = event.timestamp().hour();
            if !profile.typical_hours.is_empty() && !profile.typical_hours.contains(&hour) {
                factors.push(format!("Access at unusual hour: {}:00", hour));
                anomaly_indicators += 1;
            }

            // Check if action is unusual for this user
            let action_count = profile.typical_actions.get(event.action()).copied().unwrap_or(0);
            if action_count == 0 && profile.event_count > 50 {
                factors.push(format!("First time performing {:?}", event.action()));
                anomaly_indicators += 1;
            }

            if anomaly_indicators > 0 {
                let score = (anomaly_indicators as f64 / 2.0).min(1.0);
                return Ok(Some((score, factors)));
            }
        }

        Ok(None)
    }

    fn detect_privilege_escalation(&self, user_id: &str, event: &AuditEvent) -> Result<Option<(f64, Vec<String>)>> {
        // Detect attempts to gain unauthorized privileges
        let sensitive_actions = vec![
            AuditAction::TenantUpdated,
            AuditAction::RoleChanged,
        ];

        if sensitive_actions.contains(event.action()) && event.outcome() == &AuditOutcome::Failure {
            let recent = self.recent_events.read();
            let recent_privilege_attempts = recent.iter()
                .filter(|e| {
                    if let crate::domain::entities::Actor::User { user_id: uid, .. } = e.actor() {
                        uid == user_id && sensitive_actions.contains(e.action())
                            && (Utc::now() - e.timestamp()) < Duration::hours(1)
                    } else {
                        false
                    }
                })
                .count();

            if recent_privilege_attempts >= 3 {
                let score = 0.8;
                let factors = vec![
                    format!("{} privilege escalation attempts in 1 hour", recent_privilege_attempts),
                    format!("Latest action: {:?}", event.action()),
                ];
                return Ok(Some((score, factors)));
            }
        }

        Ok(None)
    }

    fn detect_data_exfiltration(&self, user_id: &str, event: &AuditEvent) -> Result<Option<(f64, Vec<String>)>> {
        // Detect unusual data access patterns that might indicate exfiltration
        if event.action() != &AuditAction::EventQueried {
            return Ok(None);
        }

        let recent = self.recent_events.read();
        let recent_queries = recent.iter()
            .filter(|e| {
                if let crate::domain::entities::Actor::User { user_id: uid, .. } = e.actor() {
                    uid == user_id && e.action() == &AuditAction::EventQueried
                        && (Utc::now() - e.timestamp()) < Duration::hours(1)
                } else {
                    false
                }
            })
            .count();

        // Check user profile for baseline
        let profiles = self.user_profiles.read();
        let profile_key = format!("{}-{}", event.tenant_id().as_str(), user_id);

        if let Some(profile) = profiles.get(&profile_key) {
            if profile.event_count >= self.config.read().min_baseline_events {
                // If current query rate is 5x normal, flag as anomalous
                if recent_queries as f64 > profile.avg_actions_per_hour * 5.0 {
                    let score = 0.75;
                    let factors = vec![
                        format!("{} queries in 1 hour (baseline: {:.0})", recent_queries, profile.avg_actions_per_hour),
                        "Potential data exfiltration pattern".to_string(),
                    ];
                    return Ok(Some((score, factors)));
                }
            }
        }

        Ok(None)
    }

    fn detect_velocity_anomaly(&self, user_id: &str, event: &AuditEvent) -> Result<Option<(f64, Vec<String>)>> {
        // Detect impossibly fast actions (e.g., actions from different locations in short time)
        let recent = self.recent_events.read();
        let very_recent = recent.iter()
            .filter(|e| {
                if let crate::domain::entities::Actor::User { user_id: uid, .. } = e.actor() {
                    uid == user_id && (Utc::now() - e.timestamp()) < Duration::seconds(10)
                } else {
                    false
                }
            })
            .count();

        // More than 20 actions in 10 seconds is suspicious
        if very_recent >= 20 {
            let score = 0.7;
            let factors = vec![
                format!("{} actions in 10 seconds", very_recent),
                "Potential automated attack or compromised credentials".to_string(),
            ];
            return Ok(Some((score, factors)));
        }

        Ok(None)
    }

    fn add_recent_event(&self, event: AuditEvent) {
        let mut events = self.recent_events.write();
        events.push(event);

        // Keep only recent events (last 24 hours)
        let cutoff = Utc::now() - Duration::hours(24);
        events.retain(|e| e.timestamp() > &cutoff);

        // Limit size to prevent memory issues
        if events.len() > 10000 {
            events.drain(0..1000);
        }
    }

    /// Get statistics about detection
    pub fn get_stats(&self) -> DetectionStats {
        let profiles = self.user_profiles.read();
        let recent = self.recent_events.read();

        DetectionStats {
            user_profiles_count: profiles.len(),
            recent_events_count: recent.len(),
            config: self.config.read().clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionStats {
    pub user_profiles_count: usize,
    pub recent_events_count: usize,
    pub config: AnomalyDetectionConfig,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::Actor;

    fn create_test_event(action: AuditAction, outcome: AuditOutcome, user_id: &str) -> AuditEvent {
        let tenant_id = TenantId::new("test-tenant".to_string()).unwrap();
        let actor = Actor::User {
            user_id: user_id.to_string(),
            username: "testuser".to_string(),
        };
        AuditEvent::new(tenant_id, action, actor, outcome)
    }

    #[test]
    fn test_anomaly_detector_creation() {
        let detector = AnomalyDetector::new(AnomalyDetectionConfig::default());
        let stats = detector.get_stats();
        assert_eq!(stats.user_profiles_count, 0);
        assert_eq!(stats.recent_events_count, 0);
    }

    #[test]
    fn test_normal_behavior_not_flagged() {
        let detector = AnomalyDetector::new(AnomalyDetectionConfig::default());
        let event = create_test_event(AuditAction::EventQueried, AuditOutcome::Success, "user1");

        let result = detector.analyze_event(&event).unwrap();
        assert!(!result.is_anomalous);
        assert_eq!(result.recommended_action, RecommendedAction::Monitor);
    }

    #[test]
    fn test_brute_force_detection() {
        let detector = AnomalyDetector::new(AnomalyDetectionConfig::default());

        // Simulate 6 failed login attempts
        for _ in 0..6 {
            let event = create_test_event(AuditAction::Login, AuditOutcome::Failure, "user1");
            detector.add_recent_event(event.clone());
        }

        // Next login attempt should be flagged
        let event = create_test_event(AuditAction::Login, AuditOutcome::Failure, "user1");
        let result = detector.analyze_event(&event).unwrap();

        assert!(result.is_anomalous);
        assert_eq!(result.anomaly_type, Some(AnomalyType::BruteForceAttack));
        assert!(result.score >= 0.5);
    }

    #[test]
    fn test_profile_building() {
        let detector = AnomalyDetector::new(AnomalyDetectionConfig::default());
        let event = create_test_event(AuditAction::EventQueried, AuditOutcome::Success, "user1");

        detector.update_profile(&event).unwrap();

        let stats = detector.get_stats();
        assert_eq!(stats.user_profiles_count, 1);
    }

    #[test]
    fn test_velocity_anomaly() {
        let detector = AnomalyDetector::new(AnomalyDetectionConfig::default());

        // Simulate 25 actions in rapid succession
        for _ in 0..25 {
            let event = create_test_event(AuditAction::EventQueried, AuditOutcome::Success, "user1");
            detector.add_recent_event(event.clone());
        }

        let event = create_test_event(AuditAction::EventQueried, AuditOutcome::Success, "user1");
        let result = detector.analyze_event(&event).unwrap();

        assert!(result.is_anomalous);
        assert_eq!(result.anomaly_type, Some(AnomalyType::VelocityAnomaly));
    }

    #[test]
    fn test_disabled_detection() {
        let mut config = AnomalyDetectionConfig::default();
        config.enabled = false;

        let detector = AnomalyDetector::new(config);
        let event = create_test_event(AuditAction::Login, AuditOutcome::Failure, "user1");

        let result = detector.analyze_event(&event).unwrap();
        assert!(!result.is_anomalous);
    }
}
