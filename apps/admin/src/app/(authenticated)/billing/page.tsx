"use client";

import { useCallback, useEffect, useState } from "react";
import {
  DollarSign,
  TrendingUp,
  TrendingDown,
  BarChart3,
  AlertTriangle,
  RotateCcw,
  Search,
} from "lucide-react";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Badge,
  Button,
  Input,
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
} from "@allsource/ui";
import type { ChartConfig } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { StatCard } from "@/components/monitoring/stat-card";
import {
  fetchRevenue,
  fetchInvoices,
  fetchDunning,
  processRefund,
  formatCurrency,
  formatPercent,
  type RevenueRange,
  type InvoiceStatus,
  type RevenueSummary,
  type Invoice,
  type DunningEntry,
} from "@/lib/billing-api";
import { Area, AreaChart, XAxis, YAxis, CartesianGrid } from "recharts";

// ---------------------------------------------------------------------------
// Revenue chart config
// ---------------------------------------------------------------------------

const chartConfig = {
  mrr: {
    label: "MRR",
    color: "hsl(142, 76%, 36%)",
  },
} satisfies ChartConfig;

// ---------------------------------------------------------------------------
// Refund confirmation dialog (inline — no Dialog component in packages/ui)
// ---------------------------------------------------------------------------

function RefundDialog({
  invoice,
  onConfirm,
  onCancel,
  isProcessing,
}: {
  invoice: Invoice;
  onConfirm: () => void;
  onCancel: () => void;
  isProcessing: boolean;
}) {
  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center bg-background/80 backdrop-blur-sm"
      data-testid="refund-dialog"
    >
      <Card className="w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle>Confirm Refund</CardTitle>
          <CardDescription>
            This action cannot be undone. The following invoice will be refunded.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="rounded-lg border border-border p-4 space-y-2 text-sm">
            <div className="flex justify-between">
              <span className="text-muted-foreground">Invoice</span>
              <span className="font-mono">{invoice.id}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Tenant</span>
              <span>{invoice.tenant_name}</span>
            </div>
            <div className="flex justify-between">
              <span className="text-muted-foreground">Amount</span>
              <span className="font-semibold">
                {formatCurrency(invoice.amount, invoice.currency)}
              </span>
            </div>
          </div>
          <div className="flex gap-3 justify-end">
            <Button
              variant="outline"
              onClick={onCancel}
              disabled={isProcessing}
              data-testid="refund-cancel"
            >
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={onConfirm}
              disabled={isProcessing}
              data-testid="refund-confirm"
            >
              {isProcessing ? "Processing..." : "Refund"}
            </Button>
          </div>
        </CardContent>
      </Card>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main page
// ---------------------------------------------------------------------------

export default function BillingPage() {
  // Revenue state
  const [revenue, setRevenue] = useState<RevenueSummary | null>(null);
  const [revenueRange, setRevenueRange] = useState<RevenueRange>("30d");
  const [revenueLoading, setRevenueLoading] = useState(true);

  // Invoices state
  const [invoices, setInvoices] = useState<Invoice[]>([]);
  const [invoicesTotal, setInvoicesTotal] = useState(0);
  const [invoicePage, setInvoicePage] = useState(1);
  const [statusFilter, setStatusFilter] = useState<InvoiceStatus | "">("");
  const [tenantFilter, setTenantFilter] = useState("");
  const [invoicesLoading, setInvoicesLoading] = useState(true);

  // Dunning state
  const [dunning, setDunning] = useState<DunningEntry[]>([]);
  const [dunningLoading, setDunningLoading] = useState(true);

  // Refund dialog state
  const [refundTarget, setRefundTarget] = useState<Invoice | null>(null);
  const [refundProcessing, setRefundProcessing] = useState(false);

  // ---------------------------------------------------------------------------
  // Data fetching
  // ---------------------------------------------------------------------------

  const loadRevenue = useCallback(async (range: RevenueRange) => {
    setRevenueLoading(true);
    try {
      const data = await fetchRevenue(range);
      setRevenue(data);
    } catch (err) {
      console.error("Failed to load revenue:", err);
    } finally {
      setRevenueLoading(false);
    }
  }, []);

  const loadInvoices = useCallback(
    async (page: number, status: InvoiceStatus | "", tenant: string) => {
      setInvoicesLoading(true);
      try {
        const data = await fetchInvoices({
          page,
          status: status || undefined,
          tenant_id: tenant || undefined,
        });
        setInvoices(data.invoices);
        setInvoicesTotal(data.total);
      } catch (err) {
        console.error("Failed to load invoices:", err);
      } finally {
        setInvoicesLoading(false);
      }
    },
    []
  );

  const loadDunning = useCallback(async () => {
    setDunningLoading(true);
    try {
      const data = await fetchDunning();
      setDunning(data);
    } catch (err) {
      console.error("Failed to load dunning:", err);
    } finally {
      setDunningLoading(false);
    }
  }, []);

  // Initial load
  useEffect(() => {
    loadRevenue(revenueRange);
    loadInvoices(invoicePage, statusFilter, tenantFilter);
    loadDunning();
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Reload revenue when range changes
  useEffect(() => {
    loadRevenue(revenueRange);
  }, [revenueRange, loadRevenue]);

  // Reload invoices when filters change
  useEffect(() => {
    loadInvoices(invoicePage, statusFilter, tenantFilter);
  }, [invoicePage, statusFilter, tenantFilter, loadInvoices]);

  // ---------------------------------------------------------------------------
  // Refund handler
  // ---------------------------------------------------------------------------

  const handleRefund = async () => {
    if (!refundTarget) return;
    setRefundProcessing(true);
    try {
      await processRefund(refundTarget.id);
      // Reload invoices after refund
      await loadInvoices(invoicePage, statusFilter, tenantFilter);
      setRefundTarget(null);
    } catch (err) {
      console.error("Refund failed:", err);
    } finally {
      setRefundProcessing(false);
    }
  };

  // ---------------------------------------------------------------------------
  // Status badge helper
  // ---------------------------------------------------------------------------

  const statusBadge = (status: InvoiceStatus) => {
    const variants: Record<InvoiceStatus, "default" | "secondary" | "destructive"> = {
      paid: "default",
      pending: "secondary",
      failed: "destructive",
    };
    return (
      <Badge variant={variants[status]} data-testid={`status-${status}`}>
        {status}
      </Badge>
    );
  };

  // ---------------------------------------------------------------------------
  // Render
  // ---------------------------------------------------------------------------

  return (
    <div className="space-y-6" data-testid="billing-page">
      {/* Header */}
      <div>
        <h1 className="text-3xl font-bold tracking-tight">Billing</h1>
        <p className="text-muted-foreground">
          Revenue metrics, invoices, and payment health.
        </p>
      </div>

      {/* Stat cards */}
      <div
        className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4"
        data-testid="billing-stat-cards"
      >
        <StatCard
          title="MRR"
          value={revenue ? formatCurrency(revenue.mrr) : "--"}
          icon={DollarSign}
          status="healthy"
        />
        <StatCard
          title="ARR"
          value={revenue ? formatCurrency(revenue.arr) : "--"}
          icon={BarChart3}
          status="healthy"
        />
        <StatCard
          title="Growth Rate"
          value={revenue ? formatPercent(revenue.growth_rate) : "--"}
          icon={TrendingUp}
          status={
            revenue
              ? revenue.growth_rate > 0
                ? "healthy"
                : "warning"
              : "healthy"
          }
        />
        <StatCard
          title="Churn Rate"
          value={revenue ? formatPercent(-Math.abs(revenue.churn_rate)) : "--"}
          icon={TrendingDown}
          status={
            revenue
              ? revenue.churn_rate > 5
                ? "critical"
                : revenue.churn_rate > 2
                  ? "warning"
                  : "healthy"
              : "healthy"
          }
        />
      </div>

      {/* Revenue trend chart */}
      <Card data-testid="revenue-chart">
        <CardHeader>
          <div className="flex items-center justify-between">
            <div>
              <CardTitle>Revenue Trend</CardTitle>
              <CardDescription>Monthly recurring revenue over time</CardDescription>
            </div>
            <div className="flex gap-1" data-testid="revenue-range-selector">
              {(["30d", "90d", "1y"] as RevenueRange[]).map((range) => (
                <Button
                  key={range}
                  variant={revenueRange === range ? "default" : "outline"}
                  size="sm"
                  onClick={() => setRevenueRange(range)}
                  data-testid={`range-${range}`}
                >
                  {range.toUpperCase()}
                </Button>
              ))}
            </div>
          </div>
        </CardHeader>
        <CardContent>
          {revenueLoading || !revenue?.trend?.length ? (
            <div className="flex h-[300px] items-center justify-center text-muted-foreground text-sm">
              {revenueLoading ? "Loading chart..." : "No trend data available"}
            </div>
          ) : (
            <ChartContainer config={chartConfig} className="h-[300px] w-full">
              <AreaChart data={revenue.trend}>
                <CartesianGrid strokeDasharray="3 3" />
                <XAxis
                  dataKey="date"
                  tickFormatter={(val: string) => {
                    const d = new Date(val);
                    return `${d.getMonth() + 1}/${d.getDate()}`;
                  }}
                  fontSize={12}
                />
                <YAxis
                  tickFormatter={(val: number) => `$${(val / 1000).toFixed(0)}k`}
                  fontSize={12}
                />
                <ChartTooltip
                  content={
                    <ChartTooltipContent
                      formatter={(value) => formatCurrency(value as number)}
                    />
                  }
                />
                <Area
                  type="monotone"
                  dataKey="mrr"
                  stroke="var(--color-mrr)"
                  fill="var(--color-mrr)"
                  fillOpacity={0.2}
                  strokeWidth={2}
                />
              </AreaChart>
            </ChartContainer>
          )}
        </CardContent>
      </Card>

      {/* Invoices */}
      <Card data-testid="invoices-section">
        <CardHeader>
          <CardTitle>Invoices</CardTitle>
          <CardDescription>
            View and manage tenant invoices. Showing {invoices.length} of{" "}
            {invoicesTotal} invoices.
          </CardDescription>
          <div className="flex flex-col sm:flex-row gap-3 pt-2">
            <div className="relative flex-1">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder="Filter by tenant ID..."
                value={tenantFilter}
                onChange={(e) => {
                  setTenantFilter(e.target.value);
                  setInvoicePage(1);
                }}
                className="pl-9"
                data-testid="tenant-filter"
              />
            </div>
            <Select
              value={statusFilter}
              onChange={(e) => {
                setStatusFilter(e.target.value as InvoiceStatus | "");
                setInvoicePage(1);
              }}
              className="w-full sm:w-40"
              data-testid="status-filter"
            >
              <option value="">All statuses</option>
              <option value="paid">Paid</option>
              <option value="pending">Pending</option>
              <option value="failed">Failed</option>
            </Select>
          </div>
        </CardHeader>
        <CardContent>
          {invoicesLoading ? (
            <div className="flex h-32 items-center justify-center text-muted-foreground text-sm">
              Loading invoices...
            </div>
          ) : invoices.length === 0 ? (
            <div className="flex h-32 items-center justify-center text-muted-foreground text-sm">
              No invoices found.
            </div>
          ) : (
            <>
              <Table data-testid="invoices-table">
                <TableHeader>
                  <TableRow>
                    <TableHead>Tenant</TableHead>
                    <TableHead>Amount</TableHead>
                    <TableHead>Status</TableHead>
                    <TableHead>Date</TableHead>
                    <TableHead className="text-right">Actions</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {invoices.map((inv) => (
                    <TableRow key={inv.id} data-testid={`invoice-${inv.id}`}>
                      <TableCell>
                        <div>
                          <p className="font-medium">{inv.tenant_name}</p>
                          <p className="text-xs text-muted-foreground font-mono">
                            {inv.tenant_id}
                          </p>
                        </div>
                      </TableCell>
                      <TableCell className="font-mono">
                        {formatCurrency(inv.amount, inv.currency)}
                      </TableCell>
                      <TableCell>{statusBadge(inv.status)}</TableCell>
                      <TableCell className="text-sm text-muted-foreground">
                        {new Date(inv.created_at).toLocaleDateString()}
                      </TableCell>
                      <TableCell className="text-right">
                        {inv.status === "paid" && (
                          <Button
                            variant="outline"
                            size="sm"
                            onClick={() => setRefundTarget(inv)}
                            data-testid={`refund-btn-${inv.id}`}
                          >
                            <RotateCcw className="mr-1 h-3 w-3" />
                            Refund
                          </Button>
                        )}
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

              {/* Pagination */}
              {invoicesTotal > invoices.length && (
                <div className="flex items-center justify-between pt-4">
                  <p className="text-sm text-muted-foreground">
                    Page {invoicePage}
                  </p>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      disabled={invoicePage <= 1}
                      onClick={() => setInvoicePage((p) => p - 1)}
                      data-testid="prev-page"
                    >
                      Previous
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setInvoicePage((p) => p + 1)}
                      data-testid="next-page"
                    >
                      Next
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>

      {/* Dunning */}
      <Card data-testid="dunning-section">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AlertTriangle className="h-5 w-5 text-yellow-500" />
            Failed Payments (Dunning)
          </CardTitle>
          <CardDescription>
            Tenants with failed payment attempts that require attention.
          </CardDescription>
        </CardHeader>
        <CardContent>
          {dunningLoading ? (
            <div className="flex h-32 items-center justify-center text-muted-foreground text-sm">
              Loading dunning data...
            </div>
          ) : dunning.length === 0 ? (
            <div className="flex h-32 items-center justify-center text-muted-foreground text-sm">
              No failed payments. All accounts are in good standing.
            </div>
          ) : (
            <Table data-testid="dunning-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Tenant</TableHead>
                  <TableHead>Amount Due</TableHead>
                  <TableHead>Retry Count</TableHead>
                  <TableHead>Last Attempt</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {dunning.map((entry) => (
                  <TableRow
                    key={entry.tenant_id}
                    data-testid={`dunning-${entry.tenant_id}`}
                  >
                    <TableCell>
                      <div>
                        <p className="font-medium">{entry.tenant_name}</p>
                        <p className="text-xs text-muted-foreground font-mono">
                          {entry.tenant_id}
                        </p>
                      </div>
                    </TableCell>
                    <TableCell className="font-mono">
                      {formatCurrency(entry.amount_due, entry.currency)}
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={entry.retry_count >= 3 ? "destructive" : "secondary"}
                      >
                        {entry.retry_count} retries
                      </Badge>
                    </TableCell>
                    <TableCell className="text-sm text-muted-foreground">
                      {new Date(entry.last_attempt).toLocaleString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      {/* Refund dialog */}
      {refundTarget && (
        <RefundDialog
          invoice={refundTarget}
          onConfirm={handleRefund}
          onCancel={() => setRefundTarget(null)}
          isProcessing={refundProcessing}
        />
      )}
    </div>
  );
}
