"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Select,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { CalendarClock, ChevronLeft, ChevronRight, Search, Users } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { AnalysisToolbar } from "@/components/tenants/analysis-toolbar";
import { AnomalyBadge } from "@/components/tenants/anomaly-badge";
import { FindingsDrawer, type TenantFindingRow } from "@/components/tenants/findings-drawer";
import {
  type AnalysisCategory,
  type AnalysisFinding,
  type AnalysisReport,
  type AnalysisSeverity,
  AnalysisUnavailableError,
  fetchTenantAnalysis,
  severityRank,
} from "@/lib/analysis-api";
import { fetchCatalog } from "@/lib/billing-api";
import { type AnomalySeverity, analyzeTenants, type ClientFinding } from "@/lib/tenant-anomalies";
import {
  fetchTenants,
  planLabel,
  type Tenant,
  type TenantPlan,
  type TenantStatus,
} from "@/lib/tenants-api";

type PlanOption = { value: TenantPlan | ""; label: string };

const titleCase = (s: string) => (s.length > 0 ? s.charAt(0).toUpperCase() + s.slice(1) : s);

// Canonical fallback tiers (subscription.go authority). The live options are
// driven from GET /api/v1/billing/catalog (the paid tiers) capped by the static
// enterprise end, so the filter can never drift from the catalog again (gap #4).
// This static list is the fallback when the catalog is unreachable and matches
// the offered tiers exactly. There is intentionally no `free` option: the
// product no longer offers a free plan (14-day trial, then paid/enterprise), and
// the canonical catalog already excludes it. Legacy tenants still on `free` keep
// rendering their badge via planLabel() — only the FILTER option is gone.
const FALLBACK_PLAN_OPTIONS: PlanOption[] = [
  { value: "", label: "All Plans" },
  { value: "indie", label: "Indie" },
  { value: "studio", label: "Studio" },
  { value: "scale", label: "Scale" },
  { value: "enterprise", label: "Enterprise" },
];

const STATUS_OPTIONS: { value: TenantStatus | ""; label: string }[] = [
  { value: "", label: "All Statuses" },
  { value: "active", label: "Active" },
  { value: "suspended", label: "Suspended" },
  { value: "archived", label: "Archived" },
];

const PAGE_SIZE_OPTIONS = [10, 25, 50];

// Stable keys for the fixed-length loading skeleton (avoids array-index keys).
const SKELETON_ROWS = ["s1", "s2", "s3", "s4", "s5"] as const;

function statusVariant(status: TenantStatus): "default" | "secondary" | "destructive" | "outline" {
  switch (status) {
    case "active":
      return "default";
    case "suspended":
      return "outline";
    case "archived":
      return "secondary";
    default:
      return "outline";
  }
}

function formatDate(iso: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

/** Map a client heuristic severity to the analysis severity vocabulary (1:1). */
function clientToAnalysisSeverity(s: AnomalySeverity): AnalysisSeverity {
  return s; // identical string union ("critical" | "warn" | "info")
}

/** A client ClientFinding rendered as an AnalysisFinding for the merged drawer. */
function clientFindingToAnalysis(f: ClientFinding): AnalysisFinding {
  return {
    category: f.category,
    severity: f.severity,
    code: f.code,
    title: f.title,
    detail: f.detail,
  };
}

export default function TenantsPage() {
  const router = useRouter();

  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [total, setTotal] = useState(0);
  const [isLoading, setIsLoading] = useState(true);

  const [search, setSearch] = useState("");
  const [planFilter, setPlanFilter] = useState<TenantPlan | "">("");
  const [planOptions, setPlanOptions] = useState<PlanOption[]>(FALLBACK_PLAN_OPTIONS);
  const [statusFilter, setStatusFilter] = useState<TenantStatus | "">("");
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

  // ── Analysis state ───────────────────────────────────────────────────
  // Deep findings keyed by tenant id, merged across the category buttons (each
  // run replaces only the categories it covers). fleet findings keyed by category
  // likewise. The instant client heuristics are derived (useMemo), never stored.
  const [deepReport, setDeepReport] = useState<AnalysisReport | null>(null);
  const [loadingCategory, setLoadingCategory] = useState<AnalysisCategory | "all" | null>(null);
  const [analysisUnavailable, setAnalysisUnavailable] = useState(false);
  const [analysisMessage, setAnalysisMessage] = useState<string | undefined>(undefined);
  const [showOnlyFlagged, setShowOnlyFlagged] = useState(false);
  const [findingsOpen, setFindingsOpen] = useState(false);

  const debounceRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const loadTenants = useCallback(
    async (params: {
      search: string;
      plan: TenantPlan | "";
      status: TenantStatus | "";
      page: number;
      per_page: number;
    }) => {
      setIsLoading(true);
      try {
        const data = await fetchTenants(params);
        setTenants(data.tenants);
        setTotal(data.total);
      } catch (err) {
        console.error("Failed to load tenants:", err);
        setTenants([]);
        setTotal(0);
      } finally {
        setIsLoading(false);
      }
    },
    []
  );

  // Drive the plan filter options from the billing catalog so they can't drift
  // from the canonical tiers (gap #4). Cap the catalog's paid tiers with the
  // static enterprise end. There is no `free` option — the catalog already
  // excludes it and the product no longer sells a free plan; the `t !== "free"`
  // guard stays defensive so a stray catalog `free` can never re-add it. On any
  // failure, keep the canonical fallback.
  useEffect(() => {
    let cancelled = false;
    fetchCatalog()
      .then((tiers) => {
        if (cancelled) return;
        const paid = tiers
          .map((t) => t.tier)
          .filter(
            (t): t is string =>
              typeof t === "string" && t.length > 0 && t !== "free" && t !== "enterprise"
          );
        if (paid.length === 0) return; // empty/odd catalog → keep fallback
        setPlanOptions([
          { value: "", label: "All Plans" },
          ...paid.map((t) => ({ value: t as TenantPlan, label: titleCase(t) })),
          { value: "enterprise", label: "Enterprise" },
        ]);
      })
      .catch((err) => {
        console.error("Failed to load billing catalog for plan filter:", err);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  // Load on mount and when filters/pagination change. `search` is read here but
  // intentionally NOT a dependency: it has its own debounced path
  // (handleSearchChange), so re-firing on every keystroke would double-fetch and
  // fight the debounce.
  // biome-ignore lint/correctness/useExhaustiveDependencies: search is debounced separately; including it would double-fetch on every keystroke.
  useEffect(() => {
    loadTenants({
      search,
      plan: planFilter,
      status: statusFilter,
      page,
      per_page: perPage,
    });
  }, [planFilter, statusFilter, page, perPage, loadTenants]);

  // Debounced search
  const handleSearchChange = (value: string) => {
    setSearch(value);
    if (debounceRef.current) clearTimeout(debounceRef.current);
    debounceRef.current = setTimeout(() => {
      setPage(1);
      loadTenants({
        search: value,
        plan: planFilter,
        status: statusFilter,
        page: 1,
        per_page: perPage,
      });
    }, 300);
  };

  // ── Instant client heuristics (zero backend, recomputed per render) ────
  const clientAnalysis = useMemo(() => analyzeTenants(tenants), [tenants]);

  // ── Deep findings indexed for row merge ────────────────────────────────
  const deepByTenant = useMemo(() => {
    const map = new Map<string, AnalysisFinding[]>();
    if (!deepReport) return map;
    for (const t of deepReport.tenants) {
      if (t.findings.length > 0) map.set(t.id, t.findings);
    }
    return map;
  }, [deepReport]);

  // ── Per-row worst severity (client ∪ deep) + finding titles for hover ──
  const rowState = useMemo(() => {
    const worst = new Map<string, AnalysisSeverity>();
    const counts = new Map<string, number>();
    const titles = new Map<string, string[]>();

    for (const t of tenants) {
      const client = clientAnalysis.byTenant.get(t.id) ?? [];
      const deep = deepByTenant.get(t.id) ?? [];
      const all: { severity: AnalysisSeverity; title: string }[] = [
        ...client.map((f) => ({
          severity: clientToAnalysisSeverity(f.severity),
          title: f.title,
        })),
        ...deep.map((f) => ({ severity: f.severity, title: f.title })),
      ];
      if (all.length === 0) continue;
      let w: AnalysisSeverity = "info";
      for (const f of all) {
        if (severityRank(f.severity) > severityRank(w)) w = f.severity;
      }
      worst.set(t.id, w);
      counts.set(t.id, all.length);
      titles.set(
        t.id,
        all.map((f) => f.title)
      );
    }
    return { worst, counts, titles };
  }, [tenants, clientAnalysis, deepByTenant]);

  // Tenants flagged by EITHER source (drives the "show only flagged" filter).
  const flaggedIds = useMemo(() => new Set(rowState.worst.keys()), [rowState]);

  const visibleTenants = useMemo(
    () => (showOnlyFlagged ? tenants.filter((t) => flaggedIds.has(t.id)) : tenants),
    [tenants, showOnlyFlagged, flaggedIds]
  );

  // ── Merged drawer rows (client + deep), deduped by tenant+code ─────────
  const drawerTenantFindings = useMemo<TenantFindingRow[]>(() => {
    const rows: TenantFindingRow[] = [];
    const seen = new Set<string>();
    const nameById = new Map(tenants.map((t) => [t.id, t.name] as const));

    // Deep first (richer detail + suggested_action), then client fills the gaps.
    if (deepReport) {
      for (const t of deepReport.tenants) {
        for (const f of t.findings) {
          const key = `${t.id}:${f.code}`;
          if (seen.has(key)) continue;
          seen.add(key);
          rows.push({ tenantId: t.id, tenantName: t.name, finding: f, source: "deep" });
        }
      }
    }
    for (const [tenantId, findings] of clientAnalysis.byTenant) {
      for (const f of findings) {
        const key = `${tenantId}:${f.code}`;
        if (seen.has(key)) continue;
        seen.add(key);
        rows.push({
          tenantId,
          tenantName: nameById.get(tenantId) ?? tenantId,
          finding: clientFindingToAnalysis(f),
          source: "client",
        });
      }
    }
    return rows;
  }, [tenants, clientAnalysis, deepReport]);

  // Fleet-wide findings come only from the deep scan.
  const fleetFindings = deepReport?.fleet_findings ?? [];
  const totalFindings = drawerTenantFindings.length + fleetFindings.length;

  // The created_at banner uses the largest list we have (the current page);
  // recomputed from the loaded tenants. Deep scan can corroborate via its own
  // created_at_not_real fleet finding, but the instant check stands alone.
  const fakeCreatedAt = clientAnalysis.fakeCreatedAt;

  // ── Deep-analysis runner ───────────────────────────────────────────────
  const runAnalysis = useCallback(async (category: AnalysisCategory | "all") => {
    setLoadingCategory(category);
    try {
      const report = await fetchTenantAnalysis(category === "all" ? undefined : category);
      setDeepReport((prev) => {
        // "all" replaces the whole report; a single-category run merges its
        // findings into the existing report so multiple buttons compose.
        if (category === "all" || !prev) return report;
        return mergeCategoryReport(prev, report, category);
      });
      setAnalysisUnavailable(false);
      setAnalysisMessage(undefined);
      setFindingsOpen(true);
    } catch (err) {
      if (err instanceof AnalysisUnavailableError) {
        setAnalysisUnavailable(true);
        setAnalysisMessage(err.message);
      } else {
        console.error("Tenant analysis failed:", err);
        setAnalysisUnavailable(true);
        setAnalysisMessage(
          err instanceof Error ? err.message : "Deep analysis failed unexpectedly."
        );
      }
    } finally {
      setLoadingCategory(null);
    }
  }, []);

  const totalPages = Math.max(1, Math.ceil(total / perPage));
  const canPrev = page > 1;
  const canNext = page < totalPages;

  return (
    <div className="space-y-6" data-testid="tenants-page">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Tenants</h1>
        <p className="text-muted-foreground">
          Manage tenant accounts, usage quotas, and configurations.
        </p>
      </div>

      {/* Page-level fake-created_at banner (instant, zero backend) */}
      {fakeCreatedAt.suspicious && (
        <div
          className="flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm"
          data-testid="created-at-banner"
        >
          <CalendarClock className="h-4 w-4 shrink-0 mt-0.5 text-yellow-600" />
          <div>
            <p className="font-medium">Created dates may not be real</p>
            <p className="text-muted-foreground text-xs">
              {fakeCreatedAt.count} tenants on this page share one creation date (
              {formatDate(`${fakeCreatedAt.date}T00:00:00Z`)}). This is the signature of a{" "}
              <code>time.Now()</code>-on-load bug — the column may not reflect real creation times.
            </p>
          </div>
        </div>
      )}

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="h-5 w-5" />
            All Tenants
          </CardTitle>
          <CardDescription>
            View and manage all registered tenants across the platform.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {/* Analysis toolbar (instant chips + on-demand deep scans) */}
          <AnalysisToolbar
            clientFlaggedCount={clientAnalysis.flaggedCount}
            deepFlaggedCount={deepReport ? deepByTenant.size : null}
            loadingCategory={loadingCategory}
            showOnlyFlagged={showOnlyFlagged}
            unavailable={analysisUnavailable}
            unavailableMessage={analysisMessage}
            totalFindings={totalFindings}
            onRun={runAnalysis}
            onToggleFlagged={setShowOnlyFlagged}
            onOpenFindings={() => setFindingsOpen(true)}
          />

          {/* Filters */}
          <div
            className="flex flex-col gap-3 sm:flex-row sm:items-center"
            data-testid="tenants-filters"
          >
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Search tenants..."
                value={search}
                onChange={(e) => handleSearchChange(e.target.value)}
                className="pl-9"
                data-testid="tenants-search"
              />
            </div>
            <Select
              value={planFilter}
              onChange={(e) => {
                setPlanFilter(e.target.value as TenantPlan | "");
                setPage(1);
              }}
              className="w-full sm:w-40"
              data-testid="tenants-plan-filter"
            >
              {planOptions.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </Select>
            <Select
              value={statusFilter}
              onChange={(e) => {
                setStatusFilter(e.target.value as TenantStatus | "");
                setPage(1);
              }}
              className="w-full sm:w-40"
              data-testid="tenants-status-filter"
            >
              {STATUS_OPTIONS.map((opt) => (
                <option key={opt.value} value={opt.value}>
                  {opt.label}
                </option>
              ))}
            </Select>
          </div>

          {/* Table */}
          {isLoading ? (
            <div className="space-y-3" data-testid="tenants-skeleton">
              {SKELETON_ROWS.map((key) => (
                <Skeleton key={key} className="h-12 w-full" />
              ))}
            </div>
          ) : visibleTenants.length === 0 ? (
            <div
              className="py-12 text-center text-sm text-muted-foreground"
              data-testid="tenants-empty"
            >
              {showOnlyFlagged && tenants.length > 0
                ? "No flagged tenants on this page."
                : "No tenants found."}
            </div>
          ) : (
            <div data-testid="tenants-table">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead className="w-10" />
                    <TableHead>Name</TableHead>
                    <TableHead>Plan</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-right">Events</TableHead>
                    <TableHead className="text-right">Members</TableHead>
                    <TableHead>Created</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {visibleTenants.map((tenant) => {
                    const severity = rowState.worst.get(tenant.id);
                    const count = rowState.counts.get(tenant.id) ?? 0;
                    return (
                      <TableRow
                        key={tenant.id}
                        className="cursor-pointer"
                        onClick={() => router.push(`/tenants/${tenant.id}`)}
                        data-testid={`tenant-row-${tenant.id}`}
                      >
                        <TableCell>
                          {severity && (
                            <AnomalyBadge
                              severity={severity}
                              count={count}
                              titles={rowState.titles.get(tenant.id)}
                              onClick={() => setFindingsOpen(true)}
                            />
                          )}
                        </TableCell>
                        <TableCell className="font-medium">{tenant.name}</TableCell>
                        <TableCell>
                          <Badge variant="outline">{planLabel(tenant.plan)}</Badge>
                        </TableCell>
                        <TableCell>
                          <Badge variant={statusVariant(tenant.status)}>{tenant.status}</Badge>
                        </TableCell>
                        <TableCell className="text-right">
                          {(tenant.event_count ?? 0).toLocaleString()}
                        </TableCell>
                        <TableCell className="text-right">{tenant.member_count ?? 0}</TableCell>
                        <TableCell>{formatDate(tenant.created_at)}</TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>
            </div>
          )}

          {/* Pagination */}
          <div
            className="flex items-center justify-between border-t pt-4"
            data-testid="tenants-pagination"
          >
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <span>Rows per page:</span>
              <Select
                value={String(perPage)}
                onChange={(e) => {
                  setPerPage(Number(e.target.value));
                  setPage(1);
                }}
                className="h-8 w-20"
                data-testid="tenants-page-size"
              >
                {PAGE_SIZE_OPTIONS.map((size) => (
                  <option key={size} value={String(size)}>
                    {size}
                  </option>
                ))}
              </Select>
            </div>
            <div className="flex items-center gap-2">
              <span className="text-sm text-muted-foreground" data-testid="tenants-page-info">
                Page {page} of {totalPages}
              </span>
              <Button
                variant="outline"
                size="icon"
                onClick={() => setPage((p) => Math.max(1, p - 1))}
                disabled={!canPrev}
                aria-label="Previous page"
                data-testid="tenants-prev-page"
              >
                <ChevronLeft className="h-4 w-4" />
              </Button>
              <Button
                variant="outline"
                size="icon"
                onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
                disabled={!canNext}
                aria-label="Next page"
                data-testid="tenants-next-page"
              >
                <ChevronRight className="h-4 w-4" />
              </Button>
            </div>
          </div>
        </CardContent>
      </Card>

      <FindingsDrawer
        open={findingsOpen}
        onClose={() => setFindingsOpen(false)}
        fleetFindings={fleetFindings}
        tenantFindings={drawerTenantFindings}
        generatedAt={deepReport?.generated_at}
      />
    </div>
  );
}

/**
 * Merge a single-category deep report into the existing report: for each tenant,
 * drop the prior findings of THIS category and add the new ones; same for fleet
 * findings; recompute summary counts from the union. Keeps multiple category
 * buttons composable without clobbering each other.
 */
function mergeCategoryReport(
  prev: AnalysisReport,
  next: AnalysisReport,
  category: AnalysisCategory
): AnalysisReport {
  const nextById = new Map(next.tenants.map((t) => [t.id, t] as const));
  const ids = new Set<string>([...prev.tenants.map((t) => t.id), ...next.tenants.map((t) => t.id)]);

  const tenants = [...ids].map((id) => {
    const p = prev.tenants.find((t) => t.id === id);
    const n = nextById.get(id);
    const base = n ?? p;
    if (!base) {
      // Unreachable (id came from one of the two lists) — satisfy the type checker.
      return {
        id,
        name: id,
        plan: "",
        status: "",
        event_count: 0,
        member_count: 0,
        created_at: "",
        worst_severity: "ok" as AnalysisSeverity,
        findings: [] as AnalysisFinding[],
      };
    }
    const keptPrior = (p?.findings ?? []).filter((f) => f.category !== category);
    const fresh = (n?.findings ?? []).filter((f) => f.category === category);
    const findings = [...keptPrior, ...fresh];
    let worst: AnalysisSeverity = "ok";
    for (const f of findings) {
      if (severityRank(f.severity) > severityRank(worst)) worst = f.severity;
    }
    return { ...base, findings, worst_severity: worst };
  });

  const fleet = [
    ...prev.fleet_findings.filter((f) => f.category !== category),
    ...next.fleet_findings.filter((f) => f.category === category),
  ];

  const flagged = tenants.filter((t) => t.findings.length > 0).length;
  const bySeverity: Partial<Record<Exclude<AnalysisSeverity, "ok">, number>> = {};
  const byCategory: Partial<Record<AnalysisCategory, number>> = {};
  for (const t of tenants) {
    for (const f of t.findings) {
      if (f.severity !== "ok") bySeverity[f.severity] = (bySeverity[f.severity] ?? 0) + 1;
      byCategory[f.category] = (byCategory[f.category] ?? 0) + 1;
    }
  }

  return {
    generated_at: next.generated_at || prev.generated_at,
    summary: {
      total_tenants: next.summary.total_tenants || prev.summary.total_tenants,
      flagged_tenants: flagged,
      by_category: byCategory,
      by_severity: bySeverity,
    },
    fleet_findings: fleet,
    tenants,
  };
}
