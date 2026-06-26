package dto

// admin_analysis_dto.go holds the wire shape for the read-only tenant-data
// analysis report served by GET /api/v1/admin/tenants/analyze (prompt 046).
//
// The endpoint NEVER mutates. Every Finding instead carries a SuggestedAction
// that deep-links to an ALREADY-built guarded action (reap-demo,
// backfill-usage reconcile, recovery/*, billing config-check). The admin UI
// (prompt 047) renders these findings and switches on the stable `Code`
// constants defined in usecases/analyze_tenants.go — those codes are an API
// contract, so they are named once and kept stable.

// Finding categories (the four analysis buckets). These mirror the
// ?category= query param the per-button calls use.
const (
	AnalysisCategoryDataIntegrity = "data_integrity"
	AnalysisCategoryPlanBilling   = "plan_billing"
	AnalysisCategoryLitter        = "litter"
	AnalysisCategoryUsageHealth   = "usage_health"
)

// Finding severities (ordered worst→best). worst_severity on a tenant rolls
// these up; "ok" is the no-finding sentinel used only for a tenant's roll-up.
const (
	AnalysisSeverityCritical = "critical"
	AnalysisSeverityWarn     = "warn"
	AnalysisSeverityInfo     = "info"
	AnalysisSeverityOK       = "ok"
)

// SuggestedAction kinds. "link" deep-links to an existing HTTP route the admin
// UI can navigate/POST to; "task" names an operator Taskfile command (e.g. the
// backfill-usage reconcile) for cases driven from the CLI.
const (
	AnalysisActionKindLink = "link"
	AnalysisActionKindTask = "task"
)

// SuggestedAction points the operator at an EXISTING guarded remediation for a
// finding. The analysis introduces no new mutation surface — Target is always a
// route/command that already exists.
type SuggestedAction struct {
	Label  string `json:"label"`
	Kind   string `json:"kind"`   // "link" | "task"
	Target string `json:"target"` // existing route (link) or task command (task)
}

// AnalysisFinding is one anomaly. The same shape is used for per-tenant findings
// and (with AffectedCount set) for fleet-wide findings.
type AnalysisFinding struct {
	Category        string           `json:"category"`
	Severity        string           `json:"severity"`
	Code            string           `json:"code"`
	Title           string           `json:"title"`
	Detail          string           `json:"detail"`
	AffectedCount   int              `json:"affected_count,omitempty"`
	SuggestedAction *SuggestedAction `json:"suggested_action,omitempty"`
}

// AnalysisTenant is the per-tenant slice of the report: the same identity columns
// the admin list shows, plus the tenant's worst severity and its findings.
type AnalysisTenant struct {
	ID            string            `json:"id"`
	Name          string            `json:"name"`
	Plan          string            `json:"plan"`
	Status        string            `json:"status"`
	EventCount    int64             `json:"event_count"`
	MemberCount   int               `json:"member_count"`
	CreatedAt     string            `json:"created_at"` // RFC3339; "" when Core has no real timestamp
	WorstSeverity string            `json:"worst_severity"`
	Findings      []AnalysisFinding `json:"findings"`
}

// AnalysisSummary is the headline rollup the dashboard renders above the table.
type AnalysisSummary struct {
	TotalTenants   int            `json:"total_tenants"`
	FlaggedTenants int            `json:"flagged_tenants"`
	ByCategory     map[string]int `json:"by_category"`
	BySeverity     map[string]int `json:"by_severity"`
}

// AnalysisReport is the full GET /api/v1/admin/tenants/analyze response.
type AnalysisReport struct {
	GeneratedAt   string            `json:"generated_at"` // RFC3339
	Summary       AnalysisSummary   `json:"summary"`
	FleetFindings []AnalysisFinding `json:"fleet_findings"`
	Tenants       []AnalysisTenant  `json:"tenants"`
}
