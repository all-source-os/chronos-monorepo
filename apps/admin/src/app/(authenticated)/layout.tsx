"use client";

import { cn } from "@allsource/ui/utils";
import { useRouter } from "next/navigation";
import { useEffect, useState } from "react";
import { Header } from "@/components/header";
import { Sidebar } from "@/components/sidebar";

export default function AuthenticatedLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const [isLoading, setIsLoading] = useState(true);
  const [sidebarCollapsed, setSidebarCollapsed] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  // Verify session on mount
  useEffect(() => {
    const verifySession = async () => {
      try {
        const response = await fetch("/api/auth/session");
        if (!response.ok) {
          router.push("/login");
          return;
        }
        const data = await response.json();
        if (!data.data?.user) {
          router.push("/login");
          return;
        }
      } catch {
        router.push("/login");
      } finally {
        setIsLoading(false);
      }
    };

    verifySession();
  }, [router]);

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
      />

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
    </div>
  );
}
