"use client";

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, RefreshCw } from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { CommandPalette } from "@/components/dashboard/command-palette";
import { DemoBanner } from "@/components/dashboard/demo-banner";
import { EarlyAccessBanner } from "@/components/dashboard/early-access-banner";
import { Header } from "@/components/dashboard/header";
import { HistoricalModeBanner } from "@/components/dashboard/historical-mode-banner";
import { NoticesBanner } from "@/components/dashboard/notices-banner";
import { Sidebar } from "@/components/dashboard/sidebar";
import { FeedbackWidget } from "@/components/feedback/feedback-widget";
import { TimeTravelProvider } from "@/hooks/use-time-travel";
import { useAuthStore } from "@/lib/stores/auth-store";

export default function DashboardLayout({ children }: { children: React.ReactNode }) {
  const router = useRouter();
  const { login, logout, setLoading, setError, isLoading, isAuthenticated, error } = useAuthStore();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);

  const fetchSession = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch("/api/auth/session", { cache: "no-store" });
      if (response.status === 401 || response.status === 403) {
        logout();
        router.replace("/login?redirect=/dashboard");
        return;
      }
      if (!response.ok) {
        throw new Error("Session service is temporarily unavailable.");
      }

      const data = await response.json();
      if (!data.data?.user) {
        logout();
        router.replace("/login?redirect=/dashboard");
        return;
      }

      login(data.data.user, data.data.tenant);
    } catch (sessionError) {
      setError(
        sessionError instanceof Error ? sessionError.message : "Session could not be verified."
      );
    } finally {
      setLoading(false);
    }
  }, [login, logout, router, setError, setLoading]);

  // Verify the cookie in the background. A persisted, previously verified
  // session can render the shell immediately; invalid sessions are still
  // rejected as soon as this request returns 401/403.
  useEffect(() => {
    fetchSession();
  }, [fetchSession]);

  // Keyboard shortcuts
  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    // Cmd+K to open command palette
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      setCommandPaletteOpen(true);
    }
  }, []);

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  if (isLoading && !isAuthenticated) {
    return (
      <div className="min-h-screen bg-background p-6" role="status" aria-label="Loading dashboard">
        <div className="mx-auto max-w-7xl animate-pulse space-y-6 pt-16">
          <div className="h-8 w-48 rounded bg-muted" />
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
            {["events", "streams", "projections", "latency"].map((metric) => (
              <div key={metric} className="h-32 rounded-xl border border-border bg-card" />
            ))}
          </div>
          <div className="h-72 rounded-xl border border-border bg-card" />
        </div>
      </div>
    );
  }

  return (
    <TimeTravelProvider>
      <div className="min-h-screen bg-background">
        {/* Sidebar - Desktop */}
        <div className="hidden md:block">
          <Sidebar
            collapsed={sidebarCollapsed}
            onToggle={() => setSidebarCollapsed(!sidebarCollapsed)}
          />
        </div>

        {/* Mobile sidebar overlay */}
        {mobileMenuOpen && (
          <>
            <button
              type="button"
              className="fixed inset-0 z-30 bg-background/80 backdrop-blur-sm md:hidden"
              onClick={() => setMobileMenuOpen(false)}
              aria-label="Close navigation"
            />
            <div className="fixed left-0 top-0 z-40 md:hidden">
              <Sidebar
                collapsed={false}
                onToggle={() => setMobileMenuOpen(false)}
                onNavigate={() => setMobileMenuOpen(false)}
              />
            </div>
          </>
        )}

        {/* Header */}
        <Header
          sidebarCollapsed={sidebarCollapsed}
          onMenuClick={() => setMobileMenuOpen(true)}
          onCommandPaletteOpen={() => setCommandPaletteOpen(true)}
        />

        {/* Main content */}
        <main
          className={cn(
            "min-h-screen pt-16 transition-all duration-300",
            sidebarCollapsed ? "md:ml-16" : "md:ml-64"
          )}
        >
          {/* Banners — inside main flow so they push content down */}
          <NoticesBanner />
          <DemoBanner />
          <EarlyAccessBanner />
          <HistoricalModeBanner />

          {error && (
            <div className="border-b border-amber-500/30 bg-amber-500/10 px-4 py-3 md:px-6">
              <div className="mx-auto flex max-w-7xl flex-col gap-3 text-sm sm:flex-row sm:items-center">
                <AlertTriangle className="h-4 w-4 shrink-0 text-amber-500" />
                <p className="flex-1">
                  Session verification is unavailable. Cached dashboard data remains visible.
                </p>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={fetchSession}
                  disabled={isLoading}
                >
                  <RefreshCw className={cn("mr-1.5 h-3.5 w-3.5", isLoading && "animate-spin")} />
                  Retry
                </Button>
              </div>
            </div>
          )}

          <div className="container mx-auto max-w-7xl p-4 md:p-6 lg:p-8">{children}</div>
        </main>

        {/* Command Palette */}
        <CommandPalette open={commandPaletteOpen} onClose={() => setCommandPaletteOpen(false)} />

        {/* Feedback Widget */}
        <FeedbackWidget />
      </div>
    </TimeTravelProvider>
  );
}
