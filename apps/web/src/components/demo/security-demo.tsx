"use client";

import { eventStoreClient } from "@/lib/event-store/client";
import type { Tenant } from "@/lib/event-store/types";
import { Button } from "@allsource/ui";
import { AnimatePresence, motion } from "framer-motion";
import {
  Activity,
  AlertTriangle,
  CheckCheck,
  CheckCircle2,
  Clock,
  Copy,
  Database,
  Eye,
  EyeOff,
  FileText,
  Filter as FilterIcon,
  Key,
  Lock,
  RefreshCw,
  Shield,
  TrendingDown,
  Users,
  XCircle,
  Zap,
} from "lucide-react";
import { useEffect, useState } from "react";

interface SecurityFeature {
  id: string;
  name: string;
  description: string;
  icon: React.ComponentType<{ className?: string }>;
  status: "enabled" | "disabled" | "configured";
  color: string;
  bgColor: string;
  borderColor: string;
}

const securityFeatures: SecurityFeature[] = [
  {
    id: "multi-tenancy",
    name: "Multi-Tenancy",
    description: "Isolated tenant environments with data segregation",
    icon: Users,
    status: "enabled",
    color: "text-blue-500",
    bgColor: "bg-blue-500/10",
    borderColor: "border-blue-500/20",
  },
  {
    id: "rbac",
    name: "RBAC",
    description: "Role-based access control with fine-grained permissions",
    icon: Lock,
    status: "enabled",
    color: "text-purple-500",
    bgColor: "bg-purple-500/10",
    borderColor: "border-purple-500/20",
  },
  {
    id: "jwt",
    name: "JWT Authentication",
    description: "Secure token-based authentication",
    icon: Key,
    status: "configured",
    color: "text-green-500",
    bgColor: "bg-green-500/10",
    borderColor: "border-green-500/20",
  },
  {
    id: "rate-limiting",
    name: "Rate Limiting",
    description: "Per-tenant request throttling",
    icon: TrendingDown,
    status: "enabled",
    color: "text-orange-500",
    bgColor: "bg-orange-500/10",
    borderColor: "border-orange-500/20",
  },
  {
    id: "ip-filtering",
    name: "IP Filtering",
    description: "Allowlist/blocklist IP address ranges",
    icon: FilterIcon,
    status: "configured",
    color: "text-yellow-500",
    bgColor: "bg-yellow-500/10",
    borderColor: "border-yellow-500/20",
  },
  {
    id: "encryption",
    name: "Encryption",
    description: "At-rest and in-transit data encryption",
    icon: Shield,
    status: "enabled",
    color: "text-red-500",
    bgColor: "bg-red-500/10",
    borderColor: "border-red-500/20",
  },
  {
    id: "audit-logs",
    name: "Audit Logs",
    description: "Comprehensive activity logging",
    icon: FileText,
    status: "enabled",
    color: "text-pink-500",
    bgColor: "bg-pink-500/10",
    borderColor: "border-pink-500/20",
  },
  {
    id: "api-keys",
    name: "API Keys",
    description: "Scoped API key management",
    icon: Database,
    status: "configured",
    color: "text-cyan-500",
    bgColor: "bg-cyan-500/10",
    borderColor: "border-cyan-500/20",
  },
];

interface AuditLogEntry {
  id: string;
  timestamp: string;
  action: string;
  user: string;
  tenant: string;
  resource: string;
  status: "success" | "failure";
  ip_address: string;
}

// Mock audit log data
const generateMockAuditLogs = (): AuditLogEntry[] => {
  const actions = [
    "CREATE_EVENT",
    "QUERY_EVENTS",
    "CREATE_PROJECTION",
    "UPDATE_TENANT",
    "DELETE_EVENT",
  ];
  const users = ["admin@acme.com", "dev@startup.io", "ops@enterprise.com"];
  const tenants = ["acme-corp", "startup-io", "enterprise-inc"];
  const resources = ["events", "projections", "tenants", "schemas"];

  return Array.from({ length: 10 }, (_, i) => {
    // Use modulo to ensure indices are always within bounds
    return {
      id: `audit-${i}`,
      timestamp: new Date(Date.now() - i * 60000).toISOString(),
      action: actions[i % actions.length] as string,
      user: users[i % users.length] as string,
      tenant: tenants[i % tenants.length] as string,
      resource: resources[i % resources.length] as string,
      status: Math.random() > 0.1 ? "success" : ("failure" as "success" | "failure"),
      ip_address: `192.168.${Math.floor(Math.random() * 255)}.${Math.floor(Math.random() * 255)}`,
    };
  });
};

interface PolicyTestResult {
  allowed: boolean;
  reasons: string[];
}

export function SecurityDemo() {
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [selectedFeature, setSelectedFeature] = useState<string | null>(null);
  const [showApiKey, setShowApiKey] = useState(false);
  const [copied, setCopied] = useState(false);
  const [auditLogs] = useState<AuditLogEntry[]>(generateMockAuditLogs());
  const [selectedTab, setSelectedTab] = useState<"overview" | "tenants" | "audit">("overview");
  const [policyTestResult, setPolicyTestResult] = useState<PolicyTestResult | null>(null);
  const [isTestingPolicy, setIsTestingPolicy] = useState(false);

  const mockApiKey = "chronos_demo_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx";

  const fetchTenants = async () => {
    setIsLoading(true);
    try {
      const data = await eventStoreClient.listTenants();
      setTenants(data);
    } catch (error) {
      console.error("Failed to fetch tenants:", error);
      // Use mock data for demo - Control Plane may not be running
      setTenants([
        {
          id: "tenant-demo-1",
          name: "ACME Corp",
          tier: "Enterprise",
          quotas: {
            events_per_day: 10000000,
            storage_gb: 1000,
            queries_per_hour: 100000,
            max_projections: 50,
            max_pipelines: 20,
            max_api_keys: 10,
          },
          created_at: new Date().toISOString(),
        },
        {
          id: "tenant-demo-2",
          name: "Startup Inc",
          tier: "Professional",
          quotas: {
            events_per_day: 1000000,
            storage_gb: 100,
            queries_per_hour: 10000,
            max_projections: 10,
            max_pipelines: 5,
            max_api_keys: 5,
          },
          created_at: new Date().toISOString(),
        },
        {
          id: "tenant-demo-3",
          name: "Dev Team",
          tier: "Standard",
          quotas: {
            events_per_day: 100000,
            storage_gb: 10,
            queries_per_hour: 1000,
            max_projections: 5,
            max_pipelines: 2,
            max_api_keys: 3,
          },
          created_at: new Date().toISOString(),
        },
      ]);
    } finally {
      setIsLoading(false);
    }
  };

  const createDemoTenant = async () => {
    setIsLoading(true);
    try {
      const tenantId = `tenant-${Date.now()}`;
      const response = await fetch("http://localhost:3901/api/v1/tenants", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          id: tenantId,
          name: `Demo Tenant ${tenants.length + 1}`,
          description: "Created from demo UI",
        }),
      });

      if (response.ok) {
        const newTenant = await response.json();
        // Add mock quotas for display
        setTenants((prev) => [
          ...prev,
          {
            ...newTenant,
            tier: "Standard",
            quotas: {
              events_per_day: 100000,
              storage_gb: 10,
              queries_per_hour: 1000,
              max_projections: 5,
              max_pipelines: 2,
              max_api_keys: 3,
            },
          },
        ]);
      } else {
        console.error("Failed to create tenant");
      }
    } catch (error) {
      console.error("Error creating tenant:", error);
    } finally {
      setIsLoading(false);
    }
  };

  const testPolicy = async () => {
    setIsTestingPolicy(true);
    setPolicyTestResult(null);
    try {
      const response = await fetch("http://localhost:3901/api/v1/policies/evaluate", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          user_id: "admin@acme.com",
          resource: "events",
          action: "write",
          attributes: {
            role: "admin",
            tenant: "acme-corp",
          },
        }),
      });

      if (response.ok) {
        const result = await response.json();
        setPolicyTestResult(result);
      } else {
        setPolicyTestResult({
          allowed: false,
          reasons: ["Policy evaluation failed - Control Plane may not be running"],
        });
      }
    } catch (error) {
      setPolicyTestResult({
        allowed: false,
        reasons: ["Error: Control Plane not available on port 3901"],
      });
    } finally {
      setIsTestingPolicy(false);
    }
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: fetchTenants is defined within component and only needs to run once on mount
  useEffect(() => {
    fetchTenants();
  }, []);

  const handleCopyApiKey = async () => {
    await navigator.clipboard.writeText(mockApiKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const getStatusIcon = (status: string) => {
    switch (status) {
      case "enabled":
        return <CheckCircle2 className="h-4 w-4 text-green-400" />;
      case "configured":
        return <Activity className="h-4 w-4 text-blue-400" />;
      case "disabled":
        return <XCircle className="h-4 w-4 text-slate-400" />;
      default:
        return null;
    }
  };

  const getTierColor = (tier: string) => {
    switch (tier) {
      case "Enterprise":
        return "text-purple-400 bg-purple-500/10 border-purple-500/20";
      case "Professional":
        return "text-blue-400 bg-blue-500/10 border-blue-500/20";
      case "Standard":
        return "text-green-400 bg-green-500/10 border-green-500/20";
      case "Free":
        return "text-slate-400 bg-slate-500/10 border-slate-500/20";
      default:
        return "text-slate-400 bg-slate-500/10 border-slate-500/20";
    }
  };

  const enabledFeatures = securityFeatures.filter((f) => f.status === "enabled").length;
  const configuredFeatures = securityFeatures.filter((f) => f.status === "configured").length;
  const totalSecurity = Math.round(
    ((enabledFeatures + configuredFeatures * 0.5) / securityFeatures.length) * 100
  );

  return (
    <div className="space-y-6">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-bold text-white">Enterprise Security</h2>
          <p className="text-sm text-slate-400 mt-1">
            Multi-layered security with tenant isolation and compliance
          </p>
        </div>
        <Button
          onClick={fetchTenants}
          disabled={isLoading}
          size="sm"
          variant="outline"
          className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
        >
          <motion.div
            animate={isLoading ? { rotate: 360 } : {}}
            transition={{
              duration: 1,
              repeat: isLoading ? Number.POSITIVE_INFINITY : 0,
              ease: "linear",
            }}
          >
            <RefreshCw className="h-4 w-4" />
          </motion.div>
          <span className="ml-2">Refresh</span>
        </Button>
      </div>

      {/* Security Score */}
      <motion.div
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        className="p-6 bg-gradient-to-br from-red-950/30 to-purple-950/30 rounded-xl border border-red-500/20"
      >
        <div className="flex items-center justify-between mb-4">
          <div className="flex items-center gap-3">
            <div className="p-3 bg-red-500/20 rounded-lg">
              <Shield className="h-8 w-8 text-red-400" />
            </div>
            <div>
              <h3 className="text-2xl font-bold text-white">Security Score</h3>
              <p className="text-sm text-slate-400">Overall system security posture</p>
            </div>
          </div>
          <div className="text-right">
            <p className="text-5xl font-bold text-red-400">{totalSecurity}%</p>
            <p className="text-sm text-slate-400 mt-1">
              {enabledFeatures}/{securityFeatures.length} features active
            </p>
          </div>
        </div>
        <div className="w-full bg-slate-700 rounded-full h-3 overflow-hidden">
          <motion.div
            initial={{ width: 0 }}
            animate={{ width: `${totalSecurity}%` }}
            transition={{ duration: 1, delay: 0.3 }}
            className="h-full bg-gradient-to-r from-red-500 to-purple-500 rounded-full"
          />
        </div>
      </motion.div>

      {/* Tabs */}
      <div className="flex gap-2 border-b border-slate-700">
        {[
          { id: "overview", label: "Overview", icon: Shield },
          { id: "tenants", label: "Tenants", icon: Users },
          { id: "audit", label: "Audit Logs", icon: FileText },
        ].map((tab) => (
          <button
            type="button"
            key={tab.id}
            onClick={() => setSelectedTab(tab.id as "overview" | "tenants" | "audit")}
            className={`flex items-center gap-2 px-4 py-2 text-sm font-medium transition-colors ${
              selectedTab === tab.id
                ? "text-white border-b-2 border-red-500"
                : "text-slate-400 hover:text-slate-300"
            }`}
          >
            <tab.icon className="h-4 w-4" />
            {tab.label}
          </button>
        ))}
      </div>

      {/* Overview Tab */}
      {selectedTab === "overview" && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-4"
        >
          {/* Security Features Grid */}
          <div className="grid grid-cols-2 md:grid-cols-4 gap-4">
            {securityFeatures.map((feature, index) => (
              <motion.button
                key={feature.id}
                onClick={() =>
                  setSelectedFeature(selectedFeature === feature.id ? null : feature.id)
                }
                initial={{ opacity: 0, scale: 0.9 }}
                animate={{ opacity: 1, scale: 1 }}
                transition={{ delay: index * 0.05 }}
                whileHover={{ scale: 1.05, y: -2 }}
                whileTap={{ scale: 0.95 }}
                className={`p-4 rounded-lg border transition-all ${
                  selectedFeature === feature.id
                    ? `${feature.bgColor} ${feature.borderColor}`
                    : "bg-slate-800/70 border-slate-700 hover:border-slate-600"
                }`}
              >
                <div className="flex items-center justify-between mb-2">
                  <feature.icon
                    className={`h-6 w-6 ${selectedFeature === feature.id ? feature.color : "text-slate-400"}`}
                  />
                  {getStatusIcon(feature.status)}
                </div>
                <h4 className="text-sm font-semibold text-white text-left mb-1">{feature.name}</h4>
                <p className="text-xs text-slate-400 text-left">{feature.description}</p>
              </motion.button>
            ))}
          </div>

          {/* Feature Detail */}
          <AnimatePresence>
            {selectedFeature && (
              <motion.div
                initial={{ opacity: 0, height: 0 }}
                animate={{ opacity: 1, height: "auto" }}
                exit={{ opacity: 0, height: 0 }}
                className="overflow-hidden"
              >
                {selectedFeature === "jwt" && (
                  <div className="p-6 bg-slate-800/70 rounded-lg border border-slate-600">
                    <div className="flex items-center gap-3 mb-4">
                      <Key className="h-6 w-6 text-green-500" />
                      <h3 className="text-lg font-semibold text-white">JWT Authentication</h3>
                    </div>
                    <div className="space-y-4">
                      <div>
                        <span className="text-xs text-slate-300 block mb-2">Sample API Key</span>
                        <div className="flex items-center gap-2">
                          <div className="flex-1 px-4 py-2 bg-slate-900/50 rounded border border-slate-700 text-white text-sm font-mono">
                            {showApiKey ? mockApiKey : "•".repeat(40)}
                          </div>
                          <Button
                            onClick={() => setShowApiKey(!showApiKey)}
                            size="sm"
                            variant="outline"
                            className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
                          >
                            {showApiKey ? (
                              <EyeOff className="h-4 w-4" />
                            ) : (
                              <Eye className="h-4 w-4" />
                            )}
                          </Button>
                          <Button
                            onClick={handleCopyApiKey}
                            size="sm"
                            variant="outline"
                            className="bg-slate-700 hover:bg-slate-600 text-white border-slate-600"
                          >
                            {copied ? (
                              <CheckCheck className="h-4 w-4 text-green-400" />
                            ) : (
                              <Copy className="h-4 w-4" />
                            )}
                          </Button>
                        </div>
                      </div>
                      <div className="grid grid-cols-3 gap-4">
                        <div className="p-3 bg-slate-900/50 rounded">
                          <p className="text-xs text-slate-400">Algorithm</p>
                          <p className="text-sm font-semibold text-white mt-1">HS256</p>
                        </div>
                        <div className="p-3 bg-slate-900/50 rounded">
                          <p className="text-xs text-slate-400">Expiration</p>
                          <p className="text-sm font-semibold text-white mt-1">7 days</p>
                        </div>
                        <div className="p-3 bg-slate-900/50 rounded">
                          <p className="text-xs text-slate-400">Refresh</p>
                          <p className="text-sm font-semibold text-green-400 mt-1">Enabled</p>
                        </div>
                      </div>
                    </div>
                  </div>
                )}

                {selectedFeature === "rbac" && (
                  <div className="p-6 bg-slate-800/70 rounded-lg border border-slate-600">
                    <div className="flex items-center justify-between mb-4">
                      <div className="flex items-center gap-3">
                        <Lock className="h-6 w-6 text-purple-500" />
                        <h3 className="text-lg font-semibold text-white">Policy Engine</h3>
                      </div>
                      <Button
                        onClick={testPolicy}
                        disabled={isTestingPolicy}
                        size="sm"
                        className="bg-purple-600 hover:bg-purple-700 text-white"
                      >
                        {isTestingPolicy ? "Testing..." : "Test Policy"}
                      </Button>
                    </div>
                    <div className="space-y-4">
                      <div className="p-4 bg-slate-900/50 rounded">
                        <p className="text-xs text-slate-400 mb-2">Test Request</p>
                        <div className="text-xs font-mono text-white space-y-1">
                          <p>User: admin@acme.com</p>
                          <p>Action: write to events</p>
                          <p>Attributes: role=admin, tenant=acme-corp</p>
                        </div>
                      </div>
                      {policyTestResult && (
                        <motion.div
                          initial={{ opacity: 0, y: -10 }}
                          animate={{ opacity: 1, y: 0 }}
                          className={`p-4 rounded border ${
                            policyTestResult.allowed
                              ? "bg-green-950/30 border-green-700/50"
                              : "bg-red-950/30 border-red-700/50"
                          }`}
                        >
                          <div className="flex items-center gap-2 mb-2">
                            {policyTestResult.allowed ? (
                              <CheckCircle2 className="h-5 w-5 text-green-400" />
                            ) : (
                              <XCircle className="h-5 w-5 text-red-400" />
                            )}
                            <span className="text-sm font-semibold text-white">
                              {policyTestResult.allowed ? "Access Granted" : "Access Denied"}
                            </span>
                          </div>
                          <ul className="text-xs text-slate-300 space-y-1">
                            {policyTestResult.reasons.map((reason) => (
                              <li key={reason}>• {reason}</li>
                            ))}
                          </ul>
                        </motion.div>
                      )}
                    </div>
                  </div>
                )}

                {selectedFeature === "rate-limiting" && (
                  <div className="p-6 bg-slate-800/70 rounded-lg border border-slate-600">
                    <div className="flex items-center gap-3 mb-4">
                      <TrendingDown className="h-6 w-6 text-orange-500" />
                      <h3 className="text-lg font-semibold text-white">Rate Limiting</h3>
                    </div>
                    <div className="grid grid-cols-3 gap-4">
                      <div className="p-4 bg-slate-900/50 rounded">
                        <Zap className="h-5 w-5 text-yellow-400 mb-2" />
                        <p className="text-xs text-slate-400">Requests/sec</p>
                        <p className="text-2xl font-bold text-white mt-1">1,000</p>
                      </div>
                      <div className="p-4 bg-slate-900/50 rounded">
                        <Clock className="h-5 w-5 text-blue-400 mb-2" />
                        <p className="text-xs text-slate-400">Window</p>
                        <p className="text-2xl font-bold text-white mt-1">1s</p>
                      </div>
                      <div className="p-4 bg-slate-900/50 rounded">
                        <AlertTriangle className="h-5 w-5 text-red-400 mb-2" />
                        <p className="text-xs text-slate-400">Burst</p>
                        <p className="text-2xl font-bold text-white mt-1">5,000</p>
                      </div>
                    </div>
                  </div>
                )}
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      )}

      {/* Tenants Tab */}
      {selectedTab === "tenants" && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-4"
        >
          {/* Create Tenant Button */}
          <div className="flex items-center justify-between p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <div>
              <h3 className="text-lg font-semibold text-white">Tenant Management</h3>
              <p className="text-sm text-slate-400 mt-1">
                Create and manage isolated tenant environments
              </p>
            </div>
            <Button
              onClick={createDemoTenant}
              disabled={isLoading}
              size="sm"
              className="bg-blue-600 hover:bg-blue-700 text-white"
            >
              <Users className="h-4 w-4 mr-2" />
              Create Tenant
            </Button>
          </div>

          {tenants.length === 0 ? (
            <div className="text-center py-12 bg-slate-800/50 rounded-lg border border-slate-700">
              <Users className="h-16 w-16 mx-auto text-slate-600 mb-4" />
              <p className="text-slate-400">No tenants found</p>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-4">
              {tenants.map((tenant, index) => (
                <motion.div
                  key={tenant.id}
                  initial={{ opacity: 0, x: -20 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.05 }}
                  className="p-6 bg-slate-800/70 rounded-lg border border-slate-700"
                >
                  <div className="flex items-start justify-between mb-4">
                    <div>
                      <div className="flex items-center gap-3 mb-2">
                        <h3 className="text-xl font-bold text-white">{tenant.name}</h3>
                        <span
                          className={`text-xs px-2 py-1 rounded border ${getTierColor(tenant.tier)}`}
                        >
                          {tenant.tier}
                        </span>
                      </div>
                      <p className="text-xs text-slate-400">Tenant ID: {tenant.id}</p>
                    </div>
                  </div>

                  <div className="grid grid-cols-2 md:grid-cols-6 gap-3">
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">Events/Day</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {(tenant.quotas.events_per_day / 1000000).toFixed(1)}M
                      </p>
                    </div>
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">Storage</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {tenant.quotas.storage_gb}GB
                      </p>
                    </div>
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">Queries/Hour</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {(tenant.quotas.queries_per_hour / 1000).toFixed(0)}K
                      </p>
                    </div>
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">Projections</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {tenant.quotas.max_projections}
                      </p>
                    </div>
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">Pipelines</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {tenant.quotas.max_pipelines}
                      </p>
                    </div>
                    <div className="p-3 bg-slate-900/50 rounded">
                      <p className="text-xs text-slate-400">API Keys</p>
                      <p className="text-sm font-bold text-white mt-1">
                        {tenant.quotas.max_api_keys}
                      </p>
                    </div>
                  </div>
                </motion.div>
              ))}
            </div>
          )}
        </motion.div>
      )}

      {/* Audit Logs Tab */}
      {selectedTab === "audit" && (
        <motion.div
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          className="space-y-3"
        >
          <div className="flex items-center justify-between p-4 bg-slate-800/70 rounded-lg border border-slate-600">
            <div className="flex items-center gap-3">
              <FileText className="h-5 w-5 text-pink-500" />
              <h3 className="text-lg font-semibold text-white">Recent Activity</h3>
            </div>
            <span className="text-sm text-slate-400">{auditLogs.length} entries</span>
          </div>

          <div className="space-y-2">
            {auditLogs.map((log, index) => (
              <motion.div
                key={log.id}
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ delay: index * 0.03 }}
                className={`p-4 rounded-lg border ${
                  log.status === "success"
                    ? "bg-slate-800/70 border-slate-700"
                    : "bg-red-950/30 border-red-700/50"
                }`}
              >
                <div className="flex items-start justify-between">
                  <div className="flex items-start gap-3 flex-1">
                    {log.status === "success" ? (
                      <CheckCircle2 className="h-4 w-4 text-green-400 mt-0.5" />
                    ) : (
                      <XCircle className="h-4 w-4 text-red-400 mt-0.5" />
                    )}
                    <div className="flex-1">
                      <div className="flex items-center gap-2 mb-1">
                        <span className="text-sm font-semibold text-white">{log.action}</span>
                        <span className="text-xs text-slate-500">•</span>
                        <span className="text-xs text-blue-400">{log.resource}</span>
                      </div>
                      <div className="flex items-center gap-3 text-xs text-slate-400">
                        <span>{log.user}</span>
                        <span>•</span>
                        <span>{log.tenant}</span>
                        <span>•</span>
                        <span>{log.ip_address}</span>
                      </div>
                    </div>
                  </div>
                  <span className="text-xs text-slate-500">
                    {new Date(log.timestamp).toLocaleTimeString()}
                  </span>
                </div>
              </motion.div>
            ))}
          </div>
        </motion.div>
      )}
    </div>
  );
}
