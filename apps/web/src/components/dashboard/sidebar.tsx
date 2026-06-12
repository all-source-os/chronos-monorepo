"use client";

import { Button, Icons } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  Activity,
  BarChart3,
  Brain,
  ChevronLeft,
  ChevronRight,
  CreditCard,
  ExternalLink,
  FileText,
  GitBranch,
  Key,
  LayoutDashboard,
  RotateCcw,
  Settings,
  Sparkles,
  Users,
  Zap,
} from "lucide-react";
import Link from "next/link";
import { usePathname } from "next/navigation";
import { useAuthStore } from "@/lib/stores/auth-store";
import { canonicalTier } from "@/lib/tier";

interface SidebarProps {
  collapsed: boolean;
  onToggle: () => void;
}

const navigation = [
  { name: "Overview", href: "/dashboard", icon: LayoutDashboard },
  { name: "Events", href: "/dashboard/events", icon: Activity },
  { name: "Memory", href: "/dashboard/memory", icon: Brain },
  { name: "Pipelines", href: "/dashboard/pipelines", icon: GitBranch },
  { name: "Analytics", href: "/dashboard/analytics", icon: BarChart3 },
  { name: "Demo Zone", href: "/dashboard/demo", icon: Zap },
  { name: "API Keys", href: "/dashboard/api-keys", icon: Key },
  { name: "Team", href: "/dashboard/team", icon: Users },
  { name: "Replay", href: "/dashboard/tools/replay", icon: RotateCcw },
  { name: "Audit Log", href: "/dashboard/settings/audit-log", icon: FileText },
  { name: "Billing", href: "/dashboard/billing", icon: CreditCard },
  { name: "Settings", href: "/dashboard/settings", icon: Settings },
];

export function Sidebar({ collapsed, onToggle }: SidebarProps) {
  const pathname = usePathname();
  const { tenant } = useAuthStore();

  const isFreeTier = canonicalTier(tenant?.subscription_tier) === "self-host";

  return (
    <aside
      className={cn(
        "fixed left-0 top-0 z-40 flex h-screen flex-col border-r border-border bg-background transition-all duration-300",
        collapsed ? "w-16" : "w-64"
      )}
    >
      {/* Logo */}
      <div className="flex h-16 items-center justify-between border-b border-border px-4">
        <Link href="/dashboard" className="group flex items-center gap-2">
          <Icons.logo className="h-8 w-8 shrink-0 text-primary transition-transform duration-300 group-hover:scale-110 group-hover:rotate-12" />
          {!collapsed && (
            <span className="text-xl font-semibold tracking-tight bg-gradient-to-r from-foreground to-foreground bg-clip-text transition-all duration-300 group-hover:from-primary group-hover:to-foreground group-hover:text-transparent">
              AllSource
            </span>
          )}
        </Link>
        <Button
          variant="ghost"
          size="icon"
          onClick={onToggle}
          className="h-8 w-8 shrink-0"
          aria-label={collapsed ? "Expand sidebar" : "Collapse sidebar"}
        >
          {collapsed ? <ChevronRight className="h-4 w-4" /> : <ChevronLeft className="h-4 w-4" />}
        </Button>
      </div>

      {/* Workspace Selector */}
      {!collapsed && tenant && (
        <div className="border-b border-border px-4 py-3">
          <div className="flex items-center gap-3">
            <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10 text-sm font-medium text-primary">
              {tenant.name.charAt(0).toUpperCase()}
            </div>
            <div className="flex-1 truncate">
              <p className="truncate text-sm font-medium">{tenant.name}</p>
              <p className="text-xs capitalize text-muted-foreground">
                {canonicalTier(tenant.subscription_tier)} plan
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Navigation */}
      <nav className="flex-1 space-y-1 overflow-y-auto px-2 py-4">
        {navigation.map((item) => {
          const isActive =
            item.href === "/dashboard"
              ? pathname === "/dashboard"
              : item.href === "/dashboard/settings"
                ? pathname === "/dashboard/settings"
                : pathname.startsWith(item.href);

          return (
            <Link
              key={item.name}
              href={item.href}
              className={cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:bg-muted hover:text-foreground",
                collapsed && "justify-center px-2"
              )}
              title={collapsed ? item.name : undefined}
            >
              <item.icon className="h-5 w-5 shrink-0" />
              {!collapsed && <span>{item.name}</span>}
            </Link>
          );
        })}
      </nav>

      {/* Upgrade CTA */}
      {isFreeTier && !collapsed && (
        <div className="border-t border-border p-4">
          <div className="rounded-lg bg-gradient-to-br from-primary/10 via-primary/5 to-transparent p-4">
            <div className="mb-2 flex items-center gap-2">
              <Sparkles className="h-4 w-4 text-primary" />
              <span className="text-sm font-medium">Upgrade to Team</span>
            </div>
            <p className="mb-3 text-xs text-muted-foreground">
              Get 10M events/month, unlimited streams, and MCP server access.
            </p>
            <Button size="sm" className="w-full" asChild>
              <Link href="/dashboard/billing">Upgrade now</Link>
            </Button>
          </div>
        </div>
      )}

      {/* Platform status link */}
      <div className="border-t border-border p-2">
        <a
          href="/status"
          className={cn(
            "flex items-center gap-2 rounded-lg px-3 py-2 text-xs text-muted-foreground transition-colors hover:bg-muted hover:text-foreground",
            collapsed && "justify-center px-2"
          )}
          title="Platform status"
        >
          <span className="flex h-2 w-2 shrink-0">
            <span className="absolute h-2 w-2 animate-ping rounded-full bg-green-500 opacity-60" />
            <span className="relative h-2 w-2 rounded-full bg-green-500" />
          </span>
          {!collapsed && (
            <>
              <span className="flex-1">Platform status</span>
              <ExternalLink className="h-3 w-3 shrink-0 opacity-60" />
            </>
          )}
        </a>
      </div>
    </aside>
  );
}
