"use client";

import { useState, useEffect, useCallback } from "react";
import { useRouter } from "next/navigation";
import { cn } from "@allsource/ui/utils";
import { Sidebar } from "@/components/dashboard/sidebar";
import { Header } from "@/components/dashboard/header";
import { CommandPalette } from "@/components/dashboard/command-palette";
import { EarlyAccessBanner } from "@/components/dashboard/early-access-banner";
import { useAuthStore } from "@/lib/stores/auth-store";

export default function DashboardLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const { login, setLoading, isLoading } = useAuthStore();
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);

  // Fetch session on mount
  useEffect(() => {
    const fetchSession = async () => {
      try {
        const response = await fetch("/api/auth/session");
        if (!response.ok) {
          router.push("/login");
          return;
        }
        const data = await response.json();
        if (data.data?.user) {
          login(data.data.user, data.data.tenant);
        } else {
          router.push("/login");
        }
      } catch {
        router.push("/login");
      } finally {
        setLoading(false);
      }
    };

    fetchSession();
  }, [login, router, setLoading]);

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

  if (isLoading) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="flex flex-col items-center gap-4">
          <div className="h-8 w-8 animate-spin rounded-full border-2 border-primary border-t-transparent" />
          <p className="text-sm text-muted-foreground">Loading...</p>
        </div>
      </div>
    );
  }

  return (
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
          <div
            className="fixed inset-0 z-30 bg-background/80 backdrop-blur-sm md:hidden"
            onClick={() => setMobileMenuOpen(false)}
          />
          <div className="fixed left-0 top-0 z-40 md:hidden">
            <Sidebar collapsed={false} onToggle={() => setMobileMenuOpen(false)} />
          </div>
        </>
      )}

      {/* Header */}
      <Header
        sidebarCollapsed={sidebarCollapsed}
        onMenuClick={() => setMobileMenuOpen(true)}
        onCommandPaletteOpen={() => setCommandPaletteOpen(true)}
      />

      {/* Early Access Banner */}
      <div
        className={cn(
          "fixed top-16 right-0 left-0 z-20 transition-all duration-300",
          sidebarCollapsed ? "md:left-16" : "md:left-64"
        )}
      >
        <EarlyAccessBanner />
      </div>

      {/* Main content */}
      <main
        className={cn(
          "min-h-screen pt-16 transition-all duration-300",
          sidebarCollapsed ? "md:ml-16" : "md:ml-64"
        )}
      >
        <div className="container mx-auto max-w-7xl p-4 md:p-6 lg:p-8">
          {children}
        </div>
      </main>

      {/* Command Palette */}
      <CommandPalette
        open={commandPaletteOpen}
        onClose={() => setCommandPaletteOpen(false)}
      />
    </div>
  );
}
