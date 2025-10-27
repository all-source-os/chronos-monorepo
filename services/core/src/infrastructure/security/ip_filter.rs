/// IP Address Filtering for Access Control
///
/// Provides allowlist/blocklist functionality for IP-based access control.
/// Supports both global and per-tenant IP restrictions.

use std::collections::HashSet;
use std::net::IpAddr;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::domain::value_objects::TenantId;
use crate::error::{AllSourceError, Result};

/// IP filter configuration
#[derive(Debug, Clone)]
pub struct IpFilter {
    /// Global IP allowlist (empty = allow all)
    global_allowlist: Arc<RwLock<HashSet<IpAddr>>>,

    /// Global IP blocklist
    global_blocklist: Arc<RwLock<HashSet<IpAddr>>>,

    /// Per-tenant IP allowlists
    tenant_allowlists: Arc<RwLock<std::collections::HashMap<String, HashSet<IpAddr>>>>,

    /// Per-tenant IP blocklists
    tenant_blocklists: Arc<RwLock<std::collections::HashMap<String, HashSet<IpAddr>>>>,

    /// Default action when no rules match
    default_action: FilterAction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterAction {
    Allow,
    Block,
}

#[derive(Debug, Clone)]
pub struct FilterResult {
    pub allowed: bool,
    pub reason: String,
}

impl IpFilter {
    /// Create a new IP filter with default allow-all policy
    pub fn new() -> Self {
        Self {
            global_allowlist: Arc::new(RwLock::new(HashSet::new())),
            global_blocklist: Arc::new(RwLock::new(HashSet::new())),
            tenant_allowlists: Arc::new(RwLock::new(std::collections::HashMap::new())),
            tenant_blocklists: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_action: FilterAction::Allow,
        }
    }

    /// Create a new IP filter with default block-all policy
    pub fn new_block_by_default() -> Self {
        Self {
            global_allowlist: Arc::new(RwLock::new(HashSet::new())),
            global_blocklist: Arc::new(RwLock::new(HashSet::new())),
            tenant_allowlists: Arc::new(RwLock::new(std::collections::HashMap::new())),
            tenant_blocklists: Arc::new(RwLock::new(std::collections::HashMap::new())),
            default_action: FilterAction::Block,
        }
    }

    // ========================================================================
    // Global Rules
    // ========================================================================

    /// Add IP to global allowlist
    pub fn add_to_global_allowlist(&self, ip: IpAddr) {
        self.global_allowlist.write().insert(ip);
    }

    /// Remove IP from global allowlist
    pub fn remove_from_global_allowlist(&self, ip: &IpAddr) -> bool {
        self.global_allowlist.write().remove(ip)
    }

    /// Add IP to global blocklist
    pub fn add_to_global_blocklist(&self, ip: IpAddr) {
        self.global_blocklist.write().insert(ip);
    }

    /// Remove IP from global blocklist
    pub fn remove_from_global_blocklist(&self, ip: &IpAddr) -> bool {
        self.global_blocklist.write().remove(ip)
    }

    /// Check if IP is in global allowlist
    pub fn is_in_global_allowlist(&self, ip: &IpAddr) -> bool {
        self.global_allowlist.read().contains(ip)
    }

    /// Check if IP is in global blocklist
    pub fn is_in_global_blocklist(&self, ip: &IpAddr) -> bool {
        self.global_blocklist.read().contains(ip)
    }

    // ========================================================================
    // Tenant-Specific Rules
    // ========================================================================

    /// Add IP to tenant allowlist
    pub fn add_to_tenant_allowlist(&self, tenant_id: &TenantId, ip: IpAddr) {
        let mut allowlists = self.tenant_allowlists.write();
        allowlists
            .entry(tenant_id.as_str().to_string())
            .or_insert_with(HashSet::new)
            .insert(ip);
    }

    /// Remove IP from tenant allowlist
    pub fn remove_from_tenant_allowlist(&self, tenant_id: &TenantId, ip: &IpAddr) -> bool {
        let mut allowlists = self.tenant_allowlists.write();
        if let Some(list) = allowlists.get_mut(tenant_id.as_str()) {
            list.remove(ip)
        } else {
            false
        }
    }

    /// Add IP to tenant blocklist
    pub fn add_to_tenant_blocklist(&self, tenant_id: &TenantId, ip: IpAddr) {
        let mut blocklists = self.tenant_blocklists.write();
        blocklists
            .entry(tenant_id.as_str().to_string())
            .or_insert_with(HashSet::new)
            .insert(ip);
    }

    /// Remove IP from tenant blocklist
    pub fn remove_from_tenant_blocklist(&self, tenant_id: &TenantId, ip: &IpAddr) -> bool {
        let mut blocklists = self.tenant_blocklists.write();
        if let Some(list) = blocklists.get_mut(tenant_id.as_str()) {
            list.remove(ip)
        } else {
            false
        }
    }

    // ========================================================================
    // Filtering Logic
    // ========================================================================

    /// Check if an IP is allowed (global rules only)
    pub fn is_allowed(&self, ip: &IpAddr) -> FilterResult {
        // Check global blocklist first (highest priority)
        if self.is_in_global_blocklist(ip) {
            return FilterResult {
                allowed: false,
                reason: "IP is in global blocklist".to_string(),
            };
        }

        // Check global allowlist
        let allowlist = self.global_allowlist.read();
        if !allowlist.is_empty() {
            // If allowlist exists and is not empty, IP must be in it
            if allowlist.contains(ip) {
                return FilterResult {
                    allowed: true,
                    reason: "IP is in global allowlist".to_string(),
                };
            } else {
                return FilterResult {
                    allowed: false,
                    reason: "IP not in global allowlist".to_string(),
                };
            }
        }

        // No rules matched, use default action
        match self.default_action {
            FilterAction::Allow => FilterResult {
                allowed: true,
                reason: "Default allow policy".to_string(),
            },
            FilterAction::Block => FilterResult {
                allowed: false,
                reason: "Default block policy".to_string(),
            },
        }
    }

    /// Check if an IP is allowed for a specific tenant
    pub fn is_allowed_for_tenant(&self, tenant_id: &TenantId, ip: &IpAddr) -> FilterResult {
        // Check global blocklist first (highest priority)
        if self.is_in_global_blocklist(ip) {
            return FilterResult {
                allowed: false,
                reason: "IP is in global blocklist".to_string(),
            };
        }

        // Check tenant-specific blocklist
        let tenant_blocklists = self.tenant_blocklists.read();
        if let Some(blocklist) = tenant_blocklists.get(tenant_id.as_str()) {
            if blocklist.contains(ip) {
                return FilterResult {
                    allowed: false,
                    reason: format!("IP is in blocklist for tenant {}", tenant_id.as_str()),
                };
            }
        }

        // Check tenant-specific allowlist
        let tenant_allowlists = self.tenant_allowlists.read();
        if let Some(allowlist) = tenant_allowlists.get(tenant_id.as_str()) {
            if !allowlist.is_empty() {
                // Tenant has allowlist, IP must be in it
                if allowlist.contains(ip) {
                    return FilterResult {
                        allowed: true,
                        reason: format!("IP is in allowlist for tenant {}", tenant_id.as_str()),
                    };
                } else {
                    return FilterResult {
                        allowed: false,
                        reason: format!("IP not in allowlist for tenant {}", tenant_id.as_str()),
                    };
                }
            }
        }

        // Fall back to global rules
        self.is_allowed(ip)
    }

    /// Clear all rules
    pub fn clear_all(&self) {
        self.global_allowlist.write().clear();
        self.global_blocklist.write().clear();
        self.tenant_allowlists.write().clear();
        self.tenant_blocklists.write().clear();
    }

    /// Get statistics about the filter
    pub fn stats(&self) -> IpFilterStats {
        IpFilterStats {
            global_allowlist_count: self.global_allowlist.read().len(),
            global_blocklist_count: self.global_blocklist.read().len(),
            tenant_allowlist_count: self.tenant_allowlists.read().len(),
            tenant_blocklist_count: self.tenant_blocklists.read().len(),
            default_action: self.default_action,
        }
    }
}

#[derive(Debug, Clone)]
pub struct IpFilterStats {
    pub global_allowlist_count: usize,
    pub global_blocklist_count: usize,
    pub tenant_allowlist_count: usize,
    pub tenant_blocklist_count: usize,
    pub default_action: FilterAction,
}

impl Default for IpFilter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn test_ip() -> IpAddr {
        IpAddr::from_str("192.168.1.1").unwrap()
    }

    fn test_ip2() -> IpAddr {
        IpAddr::from_str("10.0.0.1").unwrap()
    }

    fn test_tenant() -> TenantId {
        TenantId::new("test-tenant".to_string()).unwrap()
    }

    #[test]
    fn test_default_allow() {
        let filter = IpFilter::new();
        let result = filter.is_allowed(&test_ip());
        assert!(result.allowed);
        assert_eq!(result.reason, "Default allow policy");
    }

    #[test]
    fn test_default_block() {
        let filter = IpFilter::new_block_by_default();
        let result = filter.is_allowed(&test_ip());
        assert!(!result.allowed);
        assert_eq!(result.reason, "Default block policy");
    }

    #[test]
    fn test_global_allowlist() {
        let filter = IpFilter::new();
        let ip = test_ip();

        // Add to allowlist
        filter.add_to_global_allowlist(ip);

        // Should be allowed
        let result = filter.is_allowed(&ip);
        assert!(result.allowed);

        // Other IPs should be blocked (allowlist is not empty)
        let result2 = filter.is_allowed(&test_ip2());
        assert!(!result2.allowed);
    }

    #[test]
    fn test_global_blocklist() {
        let filter = IpFilter::new();
        let ip = test_ip();

        // Add to blocklist
        filter.add_to_global_blocklist(ip);

        // Should be blocked
        let result = filter.is_allowed(&ip);
        assert!(!result.allowed);
        assert_eq!(result.reason, "IP is in global blocklist");
    }

    #[test]
    fn test_blocklist_overrides_allowlist() {
        let filter = IpFilter::new();
        let ip = test_ip();

        // Add to both lists
        filter.add_to_global_allowlist(ip);
        filter.add_to_global_blocklist(ip);

        // Blocklist should take precedence
        let result = filter.is_allowed(&ip);
        assert!(!result.allowed);
    }

    #[test]
    fn test_tenant_allowlist() {
        let filter = IpFilter::new();
        let tenant = test_tenant();
        let ip = test_ip();

        // Add to tenant allowlist
        filter.add_to_tenant_allowlist(&tenant, ip);

        // Should be allowed for this tenant
        let result = filter.is_allowed_for_tenant(&tenant, &ip);
        assert!(result.allowed);

        // Other IPs should be blocked
        let result2 = filter.is_allowed_for_tenant(&tenant, &test_ip2());
        assert!(!result2.allowed);
    }

    #[test]
    fn test_tenant_blocklist() {
        let filter = IpFilter::new();
        let tenant = test_tenant();
        let ip = test_ip();

        // Add to tenant blocklist
        filter.add_to_tenant_blocklist(&tenant, ip);

        // Should be blocked for this tenant
        let result = filter.is_allowed_for_tenant(&tenant, &ip);
        assert!(!result.allowed);
    }

    #[test]
    fn test_remove_from_lists() {
        let filter = IpFilter::new();
        let ip = test_ip();

        // Add and remove from global allowlist
        filter.add_to_global_allowlist(ip);
        assert!(filter.is_in_global_allowlist(&ip));
        assert!(filter.remove_from_global_allowlist(&ip));
        assert!(!filter.is_in_global_allowlist(&ip));

        // Add and remove from global blocklist
        filter.add_to_global_blocklist(ip);
        assert!(filter.is_in_global_blocklist(&ip));
        assert!(filter.remove_from_global_blocklist(&ip));
        assert!(!filter.is_in_global_blocklist(&ip));
    }

    #[test]
    fn test_stats() {
        let filter = IpFilter::new();
        let tenant = test_tenant();

        filter.add_to_global_allowlist(test_ip());
        filter.add_to_global_blocklist(test_ip2());
        filter.add_to_tenant_allowlist(&tenant, test_ip());

        let stats = filter.stats();
        assert_eq!(stats.global_allowlist_count, 1);
        assert_eq!(stats.global_blocklist_count, 1);
        assert_eq!(stats.tenant_allowlist_count, 1);
        assert_eq!(stats.tenant_blocklist_count, 0);
    }

    #[test]
    fn test_clear_all() {
        let filter = IpFilter::new();
        let tenant = test_tenant();

        filter.add_to_global_allowlist(test_ip());
        filter.add_to_tenant_blocklist(&tenant, test_ip2());

        filter.clear_all();

        let stats = filter.stats();
        assert_eq!(stats.global_allowlist_count, 0);
        assert_eq!(stats.global_blocklist_count, 0);
        assert_eq!(stats.tenant_allowlist_count, 0);
        assert_eq!(stats.tenant_blocklist_count, 0);
    }
}
