"use client";

import { useCallback, useEffect, useRef, useState } from "react";
import { useRouter } from "next/navigation";
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
import { ChevronLeft, ChevronRight, Search, Users } from "lucide-react";
import {
  fetchTenants,
  type Tenant,
  type TenantPlan,
  type TenantStatus,
} from "@/lib/tenants-api";
import { fetchCatalog } from "@/lib/billing-api";

type PlanOption = { value: TenantPlan | ""; label: string };

const titleCase = (s: string) =>
  s.length > 0 ? s.charAt(0).toUpperCase() + s.slice(1) : s;

// Canonical fallback tiers (subscription.go authority). The live options are
// driven from GET /api/v1/billing/catalog (the paid tiers) bracketed by the
// static free/enterprise ends, so the filter can never drift from the catalog
// again (gap #4). This static list is the fallback when the catalog is
// unreachable and matches the canonical tiers exactly.
const FALLBACK_PLAN_OPTIONS: PlanOption[] = [
  { value: "", label: "All Plans" },
  { value: "free", label: "Free" },
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

function statusVariant(
  status: TenantStatus
): "default" | "secondary" | "destructive" | "outline" {
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
  return new Date(iso).toLocaleDateString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

export default function TenantsPage() {
  const router = useRouter();

  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [total, setTotal] = useState(0);
  const [isLoading, setIsLoading] = useState(true);

  const [search, setSearch] = useState("");
  const [planFilter, setPlanFilter] = useState<TenantPlan | "">("");
  const [planOptions, setPlanOptions] =
    useState<PlanOption[]>(FALLBACK_PLAN_OPTIONS);
  const [statusFilter, setStatusFilter] = useState<TenantStatus | "">("");
  const [page, setPage] = useState(1);
  const [perPage, setPerPage] = useState(10);

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
  // from the canonical tiers (gap #4). Bracket the catalog's paid tiers with the
  // static free/enterprise ends. On any failure, keep the canonical fallback.
  useEffect(() => {
    let cancelled = false;
    fetchCatalog()
      .then((tiers) => {
        if (cancelled) return;
        const paid = tiers
          .map((t) => t.tier)
          .filter(
            (t): t is string =>
              typeof t === "string" &&
              t.length > 0 &&
              t !== "free" &&
              t !== "enterprise"
          );
        if (paid.length === 0) return; // empty/odd catalog → keep fallback
        setPlanOptions([
          { value: "", label: "All Plans" },
          { value: "free", label: "Free" },
          ...paid.map(
            (t) => ({ value: t as TenantPlan, label: titleCase(t) })
          ),
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

  // Load on mount and when filters/pagination change (not search — that's debounced)
  useEffect(() => {
    loadTenants({
      search,
      plan: planFilter,
      status: statusFilter,
      page,
      per_page: perPage,
    });
  }, [planFilter, statusFilter, page, perPage, loadTenants]); // eslint-disable-line react-hooks/exhaustive-deps

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
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={`skeleton-${i}`} className="h-12 w-full" />
              ))}
            </div>
          ) : tenants.length === 0 ? (
            <div
              className="py-12 text-center text-sm text-muted-foreground"
              data-testid="tenants-empty"
            >
              No tenants found.
            </div>
          ) : (
            <div data-testid="tenants-table">
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>Name</TableHead>
                    <TableHead>Plan</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead className="text-right">Events</TableHead>
                    <TableHead className="text-right">Members</TableHead>
                    <TableHead>Created</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {tenants.map((tenant) => (
                    <TableRow
                      key={tenant.id}
                      className="cursor-pointer"
                      onClick={() => router.push(`/tenants/${tenant.id}`)}
                      data-testid={`tenant-row-${tenant.id}`}
                    >
                      <TableCell className="font-medium">
                        {tenant.name}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline">{tenant.plan}</Badge>
                      </TableCell>
                      <TableCell>
                        <Badge variant={statusVariant(tenant.status)}>
                          {tenant.status}
                        </Badge>
                      </TableCell>
                      <TableCell className="text-right">
                        {(tenant.event_count ?? 0).toLocaleString()}
                      </TableCell>
                      <TableCell className="text-right">
                        {tenant.member_count ?? 0}
                      </TableCell>
                      <TableCell>{formatDate(tenant.created_at)}</TableCell>
                    </TableRow>
                  ))}
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
              <span
                className="text-sm text-muted-foreground"
                data-testid="tenants-page-info"
              >
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
    </div>
  );
}
