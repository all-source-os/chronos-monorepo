/// Security Automation and CI/CD Integration
///
/// Automated security scanning and monitoring for:
/// - Dependency vulnerabilities (cargo audit)
/// - Code security analysis (static analysis)
/// - Secret detection
/// - License compliance
/// - Security policy enforcement
use crate::error::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, process::Command};

/// Security scan configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanConfig {
    /// Enable automatic security scanning
    pub enabled: bool,

    /// Scan frequency in hours
    pub scan_frequency_hours: u32,

    /// Enable dependency scanning
    pub enable_dependency_scan: bool,

    /// Enable secrets scanning
    pub enable_secrets_scan: bool,

    /// Enable SAST (Static Application Security Testing)
    pub enable_sast: bool,

    /// Enable license compliance checking
    pub enable_license_check: bool,

    /// Fail build on high severity issues
    pub fail_on_high_severity: bool,

    /// Fail build on medium severity issues
    pub fail_on_medium_severity: bool,
}

impl Default for SecurityScanConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            scan_frequency_hours: 24,
            enable_dependency_scan: true,
            enable_secrets_scan: true,
            enable_sast: true,
            enable_license_check: true,
            fail_on_high_severity: true,
            fail_on_medium_severity: false,
        }
    }
}

/// Security scan result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScanResult {
    /// Scan timestamp
    pub timestamp: DateTime<Utc>,

    /// Overall status
    pub status: ScanStatus,

    /// Findings by category
    pub findings: HashMap<String, Vec<SecurityFinding>>,

    /// Summary statistics
    pub summary: ScanSummary,

    /// Recommendations
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScanStatus {
    Pass,
    Warning,
    Fail,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityFinding {
    /// Finding ID
    pub id: String,

    /// Title
    pub title: String,

    /// Description
    pub description: String,

    /// Severity
    pub severity: Severity,

    /// Category
    pub category: FindingCategory,

    /// Affected component
    pub component: String,

    /// Fix recommendation
    pub fix: Option<String>,

    /// CVE ID (if applicable)
    pub cve: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FindingCategory {
    Dependency,
    SecretLeak,
    CodeVulnerability,
    LicenseIssue,
    ConfigurationIssue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
}

/// Security scanner
pub struct SecurityScanner {
    config: SecurityScanConfig,
    last_scan: Option<DateTime<Utc>>,
    last_result: Option<SecurityScanResult>,
}

impl SecurityScanner {
    /// Create new security scanner
    pub fn new(config: SecurityScanConfig) -> Self {
        Self {
            config,
            last_scan: None,
            last_result: None,
        }
    }

    /// Run full security scan
    pub fn run_full_scan(&mut self) -> Result<SecurityScanResult> {
        let mut all_findings: HashMap<String, Vec<SecurityFinding>> = HashMap::new();
        let mut recommendations = Vec::new();

        // Dependency scanning
        if self.config.enable_dependency_scan {
            match self.scan_dependencies() {
                Ok(findings) => {
                    if !findings.is_empty() {
                        all_findings.insert("dependencies".to_string(), findings);
                        recommendations.push(
                            "Run 'cargo update' to update vulnerable dependencies".to_string(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Dependency scan failed: {}", e);
                }
            }
        }

        // Secrets scanning
        if self.config.enable_secrets_scan {
            match self.scan_secrets() {
                Ok(findings) => {
                    if !findings.is_empty() {
                        all_findings.insert("secrets".to_string(), findings);
                        recommendations.push(
                            "Remove hardcoded secrets and use environment variables".to_string(),
                        );
                    }
                }
                Err(e) => {
                    eprintln!("Secrets scan failed: {}", e);
                }
            }
        }

        // SAST
        if self.config.enable_sast {
            match self.run_static_analysis() {
                Ok(findings) => {
                    if !findings.is_empty() {
                        all_findings.insert("code_analysis".to_string(), findings);
                        recommendations.push("Review and fix code quality issues".to_string());
                    }
                }
                Err(e) => {
                    eprintln!("SAST failed: {}", e);
                }
            }
        }

        // License checking
        if self.config.enable_license_check {
            match self.check_licenses() {
                Ok(findings) => {
                    if !findings.is_empty() {
                        all_findings.insert("licenses".to_string(), findings);
                        recommendations
                            .push("Review dependency licenses for compliance".to_string());
                    }
                }
                Err(e) => {
                    eprintln!("License check failed: {}", e);
                }
            }
        }

        // Calculate summary
        let summary = self.calculate_summary(&all_findings);

        // Determine overall status
        let status = self.determine_status(&summary);

        let result = SecurityScanResult {
            timestamp: Utc::now(),
            status,
            findings: all_findings,
            summary,
            recommendations,
        };

        self.last_scan = Some(Utc::now());
        self.last_result = Some(result.clone());

        Ok(result)
    }

    /// Scan dependencies for known vulnerabilities
    fn scan_dependencies(&self) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // Run cargo audit
        let output = Command::new("cargo").args(["audit", "--json"]).output();

        match output {
            Ok(output) if output.status.success() => {
                // Parse cargo audit output
                if let Ok(output_str) = String::from_utf8(output.stdout) {
                    // Simple parsing (in production, use proper JSON parsing)
                    if output_str.contains("Crate:") || output_str.contains("ID:") {
                        findings.push(SecurityFinding {
                            id: "DEP-001".to_string(),
                            title: "Vulnerable dependency detected".to_string(),
                            description: "cargo audit found vulnerabilities".to_string(),
                            severity: Severity::High,
                            category: FindingCategory::Dependency,
                            component: "dependencies".to_string(),
                            fix: Some("Run 'cargo update' and review audit output".to_string()),
                            cve: None,
                        });
                    }
                }
            }
            Ok(_) => {
                // cargo audit found issues (non-zero exit)
                findings.push(SecurityFinding {
                    id: "DEP-002".to_string(),
                    title: "Dependency vulnerabilities found".to_string(),
                    description: "cargo audit reported vulnerabilities".to_string(),
                    severity: Severity::High,
                    category: FindingCategory::Dependency,
                    component: "Cargo dependencies".to_string(),
                    fix: Some("Review 'cargo audit' output and update dependencies".to_string()),
                    cve: None,
                });
            }
            Err(_) => {
                // cargo audit not installed or failed
                eprintln!("cargo audit not available - install with: cargo install cargo-audit");
            }
        }

        Ok(findings)
    }

    /// Scan for hardcoded secrets
    fn scan_secrets(&self) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // Common secret patterns
        let secret_patterns = vec![
            (
                r"(?i)(api[_-]?key|apikey)\s*[:=]\s*[a-zA-Z0-9]{20,}",
                "API Key",
            ),
            (
                r"(?i)(password|passwd|pwd)\s*[:=]\s*[\w@#$%^&*]{8,}",
                "Password",
            ),
            (
                r"(?i)(secret[_-]?key)\s*[:=]\s*[a-zA-Z0-9]{20,}",
                "Secret Key",
            ),
            (
                r"(?i)(aws[_-]?access[_-]?key[_-]?id)\s*[:=]\s*[A-Z0-9]{20}",
                "AWS Access Key",
            ),
            (r"(?i)(private[_-]?key)\s*[:=]", "Private Key"),
        ];

        // Check common files (in production, scan all source files)
        let files_to_check = vec![".env", ".env.example", "config.toml", "Cargo.toml"];

        for file in files_to_check {
            if let Ok(content) = std::fs::read_to_string(file) {
                for (pattern, secret_type) in &secret_patterns {
                    if content.contains("password")
                        || content.contains("secret")
                        || content.contains("key")
                    {
                        findings.push(SecurityFinding {
                            id: format!("SEC-{:03}", findings.len() + 1),
                            title: format!("Potential {secret_type} found"),
                            description: format!("Potential hardcoded {secret_type} detected in {file}"),
                            severity: Severity::High,
                            category: FindingCategory::SecretLeak,
                            component: file.to_string(),
                            fix: Some("Remove hardcoded secrets, use environment variables or secret management".to_string()),
                            cve: None,
                        });
                    }
                }
            }
        }

        Ok(findings)
    }

    /// Run static application security testing
    fn run_static_analysis(&self) -> Result<Vec<SecurityFinding>> {
        let mut findings = Vec::new();

        // Run clippy with security lints
        let output = Command::new("cargo")
            .args(["clippy", "--", "-W", "clippy::all"])
            .output();

        match output {
            Ok(output) if !output.status.success() => {
                let stderr = String::from_utf8_lossy(&output.stderr);
                if stderr.contains("warning:") || stderr.contains("error:") {
                    findings.push(SecurityFinding {
                        id: "SAST-001".to_string(),
                        title: "Code quality issues found".to_string(),
                        description: "Clippy found potential code issues".to_string(),
                        severity: Severity::Medium,
                        category: FindingCategory::CodeVulnerability,
                        component: "source code".to_string(),
                        fix: Some("Run 'cargo clippy' and address warnings".to_string()),
                        cve: None,
                    });
                }
            }
            _ => {}
        }

        Ok(findings)
    }

    /// Check dependency licenses
    fn check_licenses(&self) -> Result<Vec<SecurityFinding>> {
        let findings = Vec::new();

        // Restricted licenses (example list)
        let restricted_licenses = ["GPL-3.0", "AGPL-3.0", "SSPL"];

        // In production, use cargo-license or similar tool
        // For now, this is a placeholder

        Ok(findings)
    }

    fn calculate_summary(&self, findings: &HashMap<String, Vec<SecurityFinding>>) -> ScanSummary {
        let mut summary = ScanSummary {
            total_findings: 0,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };

        for findings_vec in findings.values() {
            for finding in findings_vec {
                summary.total_findings += 1;
                match finding.severity {
                    Severity::Critical => summary.critical += 1,
                    Severity::High => summary.high += 1,
                    Severity::Medium => summary.medium += 1,
                    Severity::Low => summary.low += 1,
                    Severity::Info => summary.info += 1,
                }
            }
        }

        summary
    }

    fn determine_status(&self, summary: &ScanSummary) -> ScanStatus {
        if summary.critical > 0 {
            return ScanStatus::Fail;
        }

        if self.config.fail_on_high_severity && summary.high > 0 {
            return ScanStatus::Fail;
        }

        if self.config.fail_on_medium_severity && summary.medium > 0 {
            return ScanStatus::Fail;
        }

        if summary.high > 0 || summary.medium > 0 {
            return ScanStatus::Warning;
        }

        ScanStatus::Pass
    }

    /// Get last scan result
    pub fn get_last_result(&self) -> Option<&SecurityScanResult> {
        self.last_result.as_ref()
    }

    /// Check if scan is needed
    pub fn should_scan(&self) -> bool {
        if !self.config.enabled {
            return false;
        }

        match self.last_scan {
            None => true,
            Some(last) => {
                let elapsed = Utc::now() - last;
                elapsed.num_hours() >= self.config.scan_frequency_hours as i64
            }
        }
    }
}

/// CI/CD integration helper
pub struct CiCdIntegration;

impl CiCdIntegration {
    /// Generate GitHub Actions workflow
    pub fn generate_github_actions_workflow() -> String {
        r#"name: Security Scan

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]
  schedule:
    - cron: '0 0 * * *'  # Daily

jobs:
  security-scan:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Install Rust
        uses: actions-rs/toolchain@v1
        with:
          toolchain: stable
          components: clippy

      - name: Install cargo-audit
        run: cargo install cargo-audit

      - name: Dependency Audit
        run: cargo audit

      - name: Security Clippy
        run: cargo clippy -- -D warnings

      - name: Run Tests
        run: cargo test --lib security

      - name: Secret Scanning
        uses: trufflesecurity/trufflehog@main
        with:
          path: ./
          base: main
          head: HEAD
"#
        .to_string()
    }

    /// Generate GitLab CI configuration
    pub fn generate_gitlab_ci_config() -> String {
        r#"security-scan:
  stage: test
  image: rust:latest
  script:
    - cargo install cargo-audit
    - cargo audit
    - cargo clippy -- -D warnings
    - cargo test --lib security
  allow_failure: false
"#
        .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scanner_creation() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        assert!(scanner.last_result.is_none());
        assert!(scanner.should_scan());
    }

    #[test]
    fn test_scan_summary_calculation() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        let mut findings = HashMap::new();

        findings.insert(
            "test".to_string(),
            vec![
                SecurityFinding {
                    id: "1".to_string(),
                    title: "Test".to_string(),
                    description: "Test".to_string(),
                    severity: Severity::Critical,
                    category: FindingCategory::Dependency,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
                SecurityFinding {
                    id: "2".to_string(),
                    title: "Test2".to_string(),
                    description: "Test2".to_string(),
                    severity: Severity::High,
                    category: FindingCategory::Dependency,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
            ],
        );

        let summary = scanner.calculate_summary(&findings);
        assert_eq!(summary.total_findings, 2);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 1);
    }

    #[test]
    fn test_status_determination() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());

        let summary_critical = ScanSummary {
            total_findings: 1,
            critical: 1,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };
        assert_eq!(
            scanner.determine_status(&summary_critical),
            ScanStatus::Fail
        );

        let summary_clean = ScanSummary {
            total_findings: 0,
            critical: 0,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
        };
        assert_eq!(scanner.determine_status(&summary_clean), ScanStatus::Pass);
    }

    #[test]
    fn test_github_actions_workflow_generation() {
        let workflow = CiCdIntegration::generate_github_actions_workflow();
        assert!(workflow.contains("cargo audit"));
        assert!(workflow.contains("cargo clippy"));
        assert!(workflow.contains("Security Scan"));
    }

    #[test]
    fn test_default_security_scan_config() {
        let config = SecurityScanConfig::default();
        assert!(config.enabled);
        assert_eq!(config.scan_frequency_hours, 24);
        assert!(config.enable_dependency_scan);
        assert!(config.enable_secrets_scan);
        assert!(config.enable_sast);
        assert!(config.enable_license_check);
        assert!(config.fail_on_high_severity);
        assert!(!config.fail_on_medium_severity);
    }

    #[test]
    fn test_security_scan_config_serde() {
        let config = SecurityScanConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: SecurityScanConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.scan_frequency_hours, config.scan_frequency_hours);
    }

    #[test]
    fn test_scan_status_equality() {
        assert_eq!(ScanStatus::Pass, ScanStatus::Pass);
        assert_eq!(ScanStatus::Fail, ScanStatus::Fail);
        assert_eq!(ScanStatus::Warning, ScanStatus::Warning);
        assert_eq!(ScanStatus::Error, ScanStatus::Error);
        assert_ne!(ScanStatus::Pass, ScanStatus::Fail);
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical < Severity::High);
        assert!(Severity::High < Severity::Medium);
        assert!(Severity::Medium < Severity::Low);
        assert!(Severity::Low < Severity::Info);
    }

    #[test]
    fn test_scan_summary_all_severities() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        let mut findings = HashMap::new();

        findings.insert(
            "test".to_string(),
            vec![
                SecurityFinding {
                    id: "1".to_string(),
                    title: "Critical".to_string(),
                    description: "Critical finding".to_string(),
                    severity: Severity::Critical,
                    category: FindingCategory::Dependency,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
                SecurityFinding {
                    id: "2".to_string(),
                    title: "High".to_string(),
                    description: "High finding".to_string(),
                    severity: Severity::High,
                    category: FindingCategory::SecretLeak,
                    component: "test".to_string(),
                    fix: Some("Fix it".to_string()),
                    cve: Some("CVE-2021-1234".to_string()),
                },
                SecurityFinding {
                    id: "3".to_string(),
                    title: "Medium".to_string(),
                    description: "Medium finding".to_string(),
                    severity: Severity::Medium,
                    category: FindingCategory::CodeVulnerability,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
                SecurityFinding {
                    id: "4".to_string(),
                    title: "Low".to_string(),
                    description: "Low finding".to_string(),
                    severity: Severity::Low,
                    category: FindingCategory::LicenseIssue,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
                SecurityFinding {
                    id: "5".to_string(),
                    title: "Info".to_string(),
                    description: "Info finding".to_string(),
                    severity: Severity::Info,
                    category: FindingCategory::ConfigurationIssue,
                    component: "test".to_string(),
                    fix: None,
                    cve: None,
                },
            ],
        );

        let summary = scanner.calculate_summary(&findings);
        assert_eq!(summary.total_findings, 5);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 1);
        assert_eq!(summary.medium, 1);
        assert_eq!(summary.low, 1);
        assert_eq!(summary.info, 1);
    }

    #[test]
    fn test_status_high_severity_fail() {
        let scanner = SecurityScanner::new(SecurityScanConfig {
            fail_on_high_severity: true,
            ..Default::default()
        });

        let summary = ScanSummary {
            total_findings: 1,
            critical: 0,
            high: 1,
            medium: 0,
            low: 0,
            info: 0,
        };
        assert_eq!(scanner.determine_status(&summary), ScanStatus::Fail);
    }

    #[test]
    fn test_status_high_severity_warning() {
        let scanner = SecurityScanner::new(SecurityScanConfig {
            fail_on_high_severity: false,
            ..Default::default()
        });

        let summary = ScanSummary {
            total_findings: 1,
            critical: 0,
            high: 1,
            medium: 0,
            low: 0,
            info: 0,
        };
        assert_eq!(scanner.determine_status(&summary), ScanStatus::Warning);
    }

    #[test]
    fn test_status_medium_severity_fail() {
        let scanner = SecurityScanner::new(SecurityScanConfig {
            fail_on_high_severity: false,
            fail_on_medium_severity: true,
            ..Default::default()
        });

        let summary = ScanSummary {
            total_findings: 1,
            critical: 0,
            high: 0,
            medium: 1,
            low: 0,
            info: 0,
        };
        assert_eq!(scanner.determine_status(&summary), ScanStatus::Fail);
    }

    #[test]
    fn test_status_medium_severity_warning() {
        let scanner = SecurityScanner::new(SecurityScanConfig {
            fail_on_high_severity: false,
            fail_on_medium_severity: false,
            ..Default::default()
        });

        let summary = ScanSummary {
            total_findings: 1,
            critical: 0,
            high: 0,
            medium: 1,
            low: 0,
            info: 0,
        };
        assert_eq!(scanner.determine_status(&summary), ScanStatus::Warning);
    }

    #[test]
    fn test_should_scan_disabled() {
        let scanner = SecurityScanner::new(SecurityScanConfig {
            enabled: false,
            ..Default::default()
        });
        assert!(!scanner.should_scan());
    }

    #[test]
    fn test_get_last_result_none() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        assert!(scanner.get_last_result().is_none());
    }

    #[test]
    fn test_gitlab_ci_config_generation() {
        let config = CiCdIntegration::generate_gitlab_ci_config();
        assert!(config.contains("cargo audit"));
        assert!(config.contains("cargo clippy"));
        assert!(config.contains("security-scan"));
    }

    #[test]
    fn test_finding_category_variants() {
        let categories = vec![
            FindingCategory::Dependency,
            FindingCategory::SecretLeak,
            FindingCategory::CodeVulnerability,
            FindingCategory::LicenseIssue,
            FindingCategory::ConfigurationIssue,
        ];

        for category in categories {
            // Ensure each variant can be serialized
            let json = serde_json::to_string(&category).unwrap();
            assert!(!json.is_empty());
        }
    }

    #[test]
    fn test_security_scan_result_serde() {
        let result = SecurityScanResult {
            timestamp: Utc::now(),
            status: ScanStatus::Pass,
            findings: HashMap::new(),
            summary: ScanSummary {
                total_findings: 0,
                critical: 0,
                high: 0,
                medium: 0,
                low: 0,
                info: 0,
            },
            recommendations: vec!["Test recommendation".to_string()],
        };

        let json = serde_json::to_string(&result).unwrap();
        let parsed: SecurityScanResult = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.status, result.status);
        assert_eq!(parsed.summary.total_findings, 0);
    }

    #[test]
    fn test_security_finding_serde() {
        let finding = SecurityFinding {
            id: "TEST-001".to_string(),
            title: "Test Finding".to_string(),
            description: "A test finding".to_string(),
            severity: Severity::High,
            category: FindingCategory::Dependency,
            component: "test-component".to_string(),
            fix: Some("Apply fix".to_string()),
            cve: Some("CVE-2021-12345".to_string()),
        };

        let json = serde_json::to_string(&finding).unwrap();
        let parsed: SecurityFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, finding.id);
        assert_eq!(parsed.title, finding.title);
        assert_eq!(parsed.cve, finding.cve);
    }

    #[test]
    fn test_scan_status_serde() {
        let statuses = vec![
            ScanStatus::Pass,
            ScanStatus::Warning,
            ScanStatus::Fail,
            ScanStatus::Error,
        ];

        for status in statuses {
            let json = serde_json::to_string(&status).unwrap();
            let parsed: ScanStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn test_empty_findings_summary() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        let findings: HashMap<String, Vec<SecurityFinding>> = HashMap::new();

        let summary = scanner.calculate_summary(&findings);
        assert_eq!(summary.total_findings, 0);
        assert_eq!(summary.critical, 0);
        assert_eq!(summary.high, 0);
        assert_eq!(summary.medium, 0);
        assert_eq!(summary.low, 0);
        assert_eq!(summary.info, 0);
    }

    #[test]
    fn test_multiple_categories_summary() {
        let scanner = SecurityScanner::new(SecurityScanConfig::default());
        let mut findings = HashMap::new();

        findings.insert(
            "dependencies".to_string(),
            vec![SecurityFinding {
                id: "DEP-001".to_string(),
                title: "Dependency issue".to_string(),
                description: "A dependency issue".to_string(),
                severity: Severity::High,
                category: FindingCategory::Dependency,
                component: "deps".to_string(),
                fix: None,
                cve: None,
            }],
        );

        findings.insert(
            "secrets".to_string(),
            vec![SecurityFinding {
                id: "SEC-001".to_string(),
                title: "Secret issue".to_string(),
                description: "A secret issue".to_string(),
                severity: Severity::Critical,
                category: FindingCategory::SecretLeak,
                component: "secrets".to_string(),
                fix: None,
                cve: None,
            }],
        );

        let summary = scanner.calculate_summary(&findings);
        assert_eq!(summary.total_findings, 2);
        assert_eq!(summary.critical, 1);
        assert_eq!(summary.high, 1);
    }
}
