"use client";

import { useCallback, useEffect, useState } from "react";
import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
  Input,
  Label,
  Select,
  Skeleton,
  Switch,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@allsource/ui";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
} from "recharts";
import { Plus, ShieldCheck, Trash2, FileText, KeyRound } from "lucide-react";
import {
  fetchIpRules,
  createIpRule,
  deleteIpRule,
  fetchTokenAudit,
  fetchTokenAuditSummary,
  fetchPolicies,
  type IpRule,
  type TokenAuditEntry,
  type TokenAuditSummaryEntry,
  type RbacPolicy,
} from "@/lib/security-api";

// ---------------------------------------------------------------------------
// IP Rules Tab
// ---------------------------------------------------------------------------

function IpRulesTab() {
  const [rules, setRules] = useState<IpRule[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showAddDialog, setShowAddDialog] = useState(false);
  const [deleteTarget, setDeleteTarget] = useState<IpRule | null>(null);

  // Add form state
  const [newCidr, setNewCidr] = useState("");
  const [newType, setNewType] = useState<"allow" | "deny">("allow");
  const [newDescription, setNewDescription] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);

  const loadRules = useCallback(async () => {
    setIsLoading(true);
    try {
      const data = await fetchIpRules();
      setRules(data);
    } catch (err) {
      console.error("Failed to load IP rules:", err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => {
    loadRules();
  }, [loadRules]);

  const handleAdd = async () => {
    if (!newCidr.trim()) return;
    setIsSubmitting(true);
    try {
      await createIpRule({
        cidr: newCidr.trim(),
        rule_type: newType,
        description: newDescription.trim(),
      });
      setShowAddDialog(false);
      setNewCidr("");
      setNewType("allow");
      setNewDescription("");
      await loadRules();
    } catch (err) {
      console.error("Failed to create IP rule:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleDelete = async () => {
    if (!deleteTarget) return;
    try {
      await deleteIpRule(deleteTarget.id);
      setDeleteTarget(null);
      await loadRules();
    } catch (err) {
      console.error("Failed to delete IP rule:", err);
    }
  };

  return (
    <div className="space-y-4" data-testid="ip-rules-tab">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">IP Rules</h2>
        <Button
          onClick={() => setShowAddDialog(true)}
          data-testid="add-ip-rule-btn"
        >
          <Plus className="mr-2 h-4 w-4" />
          Add Rule
        </Button>
      </div>

      {isLoading ? (
        <div className="space-y-2" data-testid="ip-rules-skeleton">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : rules.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center">
            <p className="text-sm text-muted-foreground" data-testid="ip-rules-empty">
              No IP rules configured.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="rounded-md border" data-testid="ip-rules-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>CIDR</TableHead>
                <TableHead>Type</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Created</TableHead>
                <TableHead className="w-[80px]">Actions</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {rules.map((rule) => (
                <TableRow key={rule.id} data-testid={`ip-rule-${rule.id}`}>
                  <TableCell className="font-mono text-sm">
                    {rule.cidr}
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={
                        rule.rule_type === "allow" ? "default" : "destructive"
                      }
                    >
                      {rule.rule_type}
                    </Badge>
                  </TableCell>
                  <TableCell>{rule.description}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(rule.created_at).toLocaleDateString()}
                  </TableCell>
                  <TableCell>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setDeleteTarget(rule)}
                      data-testid={`delete-ip-rule-${rule.id}`}
                      aria-label={`Delete rule ${rule.cidr}`}
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Add Rule Dialog */}
      <Dialog open={showAddDialog} onOpenChange={setShowAddDialog}>
        <DialogContent
          onClose={() => setShowAddDialog(false)}
          data-testid="add-ip-rule-dialog"
        >
          <DialogHeader>
            <DialogTitle>Add IP Rule</DialogTitle>
            <DialogDescription>
              Create a new IP allow/deny rule by specifying a CIDR range.
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="cidr">CIDR Range</Label>
              <Input
                id="cidr"
                placeholder="e.g. 192.168.1.0/24"
                value={newCidr}
                onChange={(e) => setNewCidr(e.target.value)}
                data-testid="ip-rule-cidr-input"
              />
            </div>
            <div className="flex items-center gap-3">
              <Label htmlFor="rule-type">Rule Type</Label>
              <div className="flex items-center gap-2">
                <span
                  className={`text-sm ${newType === "deny" ? "font-medium" : "text-muted-foreground"}`}
                >
                  Deny
                </span>
                <Switch
                  id="rule-type"
                  checked={newType === "allow"}
                  onCheckedChange={(checked: boolean) =>
                    setNewType(checked ? "allow" : "deny")
                  }
                  data-testid="ip-rule-type-toggle"
                />
                <span
                  className={`text-sm ${newType === "allow" ? "font-medium" : "text-muted-foreground"}`}
                >
                  Allow
                </span>
              </div>
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">Description</Label>
              <Input
                id="description"
                placeholder="Optional description"
                value={newDescription}
                onChange={(e) => setNewDescription(e.target.value)}
                data-testid="ip-rule-description-input"
              />
            </div>
          </div>
          <DialogFooter>
            <Button
              variant="outline"
              onClick={() => setShowAddDialog(false)}
            >
              Cancel
            </Button>
            <Button
              onClick={handleAdd}
              disabled={!newCidr.trim() || isSubmitting}
              data-testid="ip-rule-submit-btn"
            >
              {isSubmitting ? "Adding..." : "Add Rule"}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* Delete Confirmation Dialog */}
      <Dialog
        open={deleteTarget !== null}
        onOpenChange={(open) => {
          if (!open) setDeleteTarget(null);
        }}
      >
        <DialogContent
          onClose={() => setDeleteTarget(null)}
          data-testid="delete-ip-rule-dialog"
        >
          <DialogHeader>
            <DialogTitle>Delete IP Rule</DialogTitle>
            <DialogDescription>
              Are you sure you want to delete the rule for{" "}
              <span className="font-mono font-medium">
                {deleteTarget?.cidr}
              </span>
              ? This action cannot be undone.
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDeleteTarget(null)}>
              Cancel
            </Button>
            <Button
              variant="destructive"
              onClick={handleDelete}
              data-testid="confirm-delete-btn"
            >
              Delete
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Token Audit Tab
// ---------------------------------------------------------------------------

function TokenAuditTab() {
  const [entries, setEntries] = useState<TokenAuditEntry[]>([]);
  const [summary, setSummary] = useState<TokenAuditSummaryEntry[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [tenantFilter, setTenantFilter] = useState("");
  const [dateFrom, setDateFrom] = useState("");
  const [dateTo, setDateTo] = useState("");
  const [viewMode, setViewMode] = useState<"table" | "chart">("table");

  const loadAudit = useCallback(async () => {
    setIsLoading(true);
    try {
      const [auditData, summaryData] = await Promise.all([
        fetchTokenAudit({
          tenant_id: tenantFilter || undefined,
          from: dateFrom || undefined,
          to: dateTo || undefined,
        }),
        fetchTokenAuditSummary(),
      ]);
      setEntries(auditData.entries);
      setSummary(summaryData);
    } catch (err) {
      console.error("Failed to load token audit:", err);
    } finally {
      setIsLoading(false);
    }
  }, [tenantFilter, dateFrom, dateTo]);

  useEffect(() => {
    loadAudit();
  }, [loadAudit]);

  return (
    <div className="space-y-4" data-testid="token-audit-tab">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">Token Audit</h2>
        <div className="flex items-center gap-2">
          <Button
            variant={viewMode === "table" ? "default" : "outline"}
            size="sm"
            onClick={() => setViewMode("table")}
            data-testid="audit-view-table"
          >
            Table
          </Button>
          <Button
            variant={viewMode === "chart" ? "default" : "outline"}
            size="sm"
            onClick={() => setViewMode("chart")}
            data-testid="audit-view-chart"
          >
            Chart
          </Button>
        </div>
      </div>

      {/* Filters */}
      <div
        className="flex flex-wrap items-end gap-3"
        data-testid="audit-filters"
      >
        <div className="space-y-1">
          <Label htmlFor="tenant-filter" className="text-xs">
            Tenant ID
          </Label>
          <Input
            id="tenant-filter"
            placeholder="Filter by tenant..."
            value={tenantFilter}
            onChange={(e) => setTenantFilter(e.target.value)}
            className="w-48"
            data-testid="audit-tenant-filter"
          />
        </div>
        <div className="space-y-1">
          <Label htmlFor="date-from" className="text-xs">
            From
          </Label>
          <Input
            id="date-from"
            type="date"
            value={dateFrom}
            onChange={(e) => setDateFrom(e.target.value)}
            className="w-40"
            data-testid="audit-date-from"
          />
        </div>
        <div className="space-y-1">
          <Label htmlFor="date-to" className="text-xs">
            To
          </Label>
          <Input
            id="date-to"
            type="date"
            value={dateTo}
            onChange={(e) => setDateTo(e.target.value)}
            className="w-40"
            data-testid="audit-date-to"
          />
        </div>
      </div>

      {isLoading ? (
        <div className="space-y-2" data-testid="audit-skeleton">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : viewMode === "table" ? (
        entries.length === 0 ? (
          <Card>
            <CardContent className="py-8 text-center">
              <p
                className="text-sm text-muted-foreground"
                data-testid="audit-empty"
              >
                No audit entries found.
              </p>
            </CardContent>
          </Card>
        ) : (
          <div className="rounded-md border" data-testid="audit-table">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Tenant</TableHead>
                  <TableHead>Token</TableHead>
                  <TableHead>Action</TableHead>
                  <TableHead>IP Address</TableHead>
                  <TableHead>Timestamp</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {entries.map((entry) => (
                  <TableRow
                    key={entry.id}
                    data-testid={`audit-entry-${entry.id}`}
                  >
                    <TableCell>{entry.tenant_name}</TableCell>
                    <TableCell className="font-mono text-sm">
                      {entry.token_prefix}...
                    </TableCell>
                    <TableCell>
                      <Badge variant="outline">{entry.action}</Badge>
                    </TableCell>
                    <TableCell className="font-mono text-sm">
                      {entry.ip_address}
                    </TableCell>
                    <TableCell className="text-muted-foreground">
                      {new Date(entry.timestamp).toLocaleString()}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )
      ) : (
        <Card data-testid="audit-chart">
          <CardHeader>
            <CardTitle className="text-base">API Calls per Tenant</CardTitle>
            <CardDescription>
              Aggregated token usage across tenants
            </CardDescription>
          </CardHeader>
          <CardContent>
            {summary.length === 0 ? (
              <p
                className="text-sm text-muted-foreground text-center py-8"
                data-testid="audit-chart-empty"
              >
                No summary data available.
              </p>
            ) : (
              <div className="h-[300px]">
                <ResponsiveContainer width="100%" height="100%">
                  <BarChart data={summary}>
                    <CartesianGrid strokeDasharray="3 3" />
                    <XAxis dataKey="tenant_name" />
                    <YAxis />
                    <Tooltip />
                    <Bar
                      dataKey="api_calls"
                      fill="hsl(142, 76%, 36%)"
                      radius={[4, 4, 0, 0]}
                    />
                  </BarChart>
                </ResponsiveContainer>
              </div>
            )}
          </CardContent>
        </Card>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// RBAC Policies Tab
// ---------------------------------------------------------------------------

function RbacPoliciesTab() {
  const [policies, setPolicies] = useState<RbacPolicy[]>([]);
  const [isLoading, setIsLoading] = useState(true);

  useEffect(() => {
    const load = async () => {
      setIsLoading(true);
      try {
        const data = await fetchPolicies();
        setPolicies(data);
      } catch (err) {
        console.error("Failed to load policies:", err);
      } finally {
        setIsLoading(false);
      }
    };
    load();
  }, []);

  return (
    <div className="space-y-4" data-testid="rbac-policies-tab">
      <div className="flex items-center justify-between">
        <h2 className="text-lg font-semibold">RBAC Policies</h2>
        <Badge variant="outline">Read-only</Badge>
      </div>

      {isLoading ? (
        <div className="space-y-2" data-testid="policies-skeleton">
          <Skeleton className="h-10 w-full" />
          <Skeleton className="h-10 w-full" />
        </div>
      ) : policies.length === 0 ? (
        <Card>
          <CardContent className="py-8 text-center">
            <p
              className="text-sm text-muted-foreground"
              data-testid="policies-empty"
            >
              No policies defined.
            </p>
          </CardContent>
        </Card>
      ) : (
        <div className="rounded-md border" data-testid="policies-table">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Name</TableHead>
                <TableHead>Description</TableHead>
                <TableHead>Permissions</TableHead>
                <TableHead>Created</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {policies.map((policy) => (
                <TableRow
                  key={policy.id}
                  data-testid={`policy-${policy.id}`}
                >
                  <TableCell className="font-medium">{policy.name}</TableCell>
                  <TableCell>{policy.description}</TableCell>
                  <TableCell>
                    <div className="flex flex-wrap gap-1">
                      {policy.permissions.map((perm) => (
                        <Badge key={perm} variant="secondary" className="text-xs">
                          {perm}
                        </Badge>
                      ))}
                    </div>
                  </TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(policy.created_at).toLocaleDateString()}
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Security Page
// ---------------------------------------------------------------------------

export default function SecurityPage() {
  const [activeTab, setActiveTab] = useState("ip-rules");

  return (
    <div className="space-y-6" data-testid="security-page">
      <div>
        <h1 className="text-3xl font-bold tracking-tight flex items-center gap-2">
          <ShieldCheck className="h-8 w-8" />
          Security
        </h1>
        <p className="text-muted-foreground">
          IP rules, token audit logs, and RBAC policy management.
        </p>
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList data-testid="security-tabs">
          <TabsTrigger value="ip-rules" data-testid="tab-ip-rules">
            <ShieldCheck className="mr-2 h-4 w-4" />
            IP Rules
          </TabsTrigger>
          <TabsTrigger value="token-audit" data-testid="tab-token-audit">
            <KeyRound className="mr-2 h-4 w-4" />
            Token Audit
          </TabsTrigger>
          <TabsTrigger value="rbac-policies" data-testid="tab-rbac-policies">
            <FileText className="mr-2 h-4 w-4" />
            RBAC Policies
          </TabsTrigger>
        </TabsList>

        <TabsContent value="ip-rules">
          <IpRulesTab />
        </TabsContent>

        <TabsContent value="token-audit">
          <TokenAuditTab />
        </TabsContent>

        <TabsContent value="rbac-policies">
          <RbacPoliciesTab />
        </TabsContent>
      </Tabs>
    </div>
  );
}
