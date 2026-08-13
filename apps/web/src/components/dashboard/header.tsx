"use client";

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { LogOut, Menu, Moon, Search, Settings, Sun } from "lucide-react";
import Image from "next/image";
import { useRouter } from "next/navigation";
import { useTheme } from "next-themes";
import { useState } from "react";
import { useAuthStore } from "@/lib/stores/auth-store";
import { TimeTravelPicker } from "./time-travel-picker";

interface HeaderProps {
  sidebarCollapsed: boolean;
  onMenuClick: () => void;
  onCommandPaletteOpen: () => void;
}

export function Header({ sidebarCollapsed, onMenuClick, onCommandPaletteOpen }: HeaderProps) {
  const router = useRouter();
  const { theme, setTheme } = useTheme();
  const { user, logout } = useAuthStore();
  const [showUserMenu, setShowUserMenu] = useState(false);

  const handleLogout = async () => {
    try {
      await fetch("/api/auth/session", { method: "DELETE" });
    } catch (error) {
      console.error("Logout failed:", error);
    } finally {
      // Clear in-memory + persisted auth state, then HARD-navigate. A soft
      // router.push left the zustand "auth-storage" localStorage entry
      // (isAuthenticated:true) in place, so navigating back to /dashboard
      // re-hydrated the old session and "Log out" appeared to do nothing.
      logout();
      try {
        localStorage.removeItem("auth-storage");
      } catch {
        // ignore (privacy mode / SSR)
      }
      window.location.href = "/login";
    }
  };

  return (
    <header
      className={cn(
        "fixed left-0 top-0 z-30 flex h-16 items-center justify-between border-b border-border bg-background/95 px-4 backdrop-blur transition-all duration-300 md:px-6",
        sidebarCollapsed ? "md:left-16" : "md:left-64"
      )}
      style={{ right: 0 }}
    >
      {/* Left section */}
      <div className="flex items-center gap-4">
        {/* Mobile menu button */}
        <Button
          variant="ghost"
          size="icon"
          className="md:hidden"
          onClick={onMenuClick}
          aria-label="Toggle menu"
        >
          <Menu className="h-5 w-5" />
        </Button>

        <Button
          variant="ghost"
          size="icon"
          className="md:hidden"
          onClick={onCommandPaletteOpen}
          aria-label="Search events and navigation"
        >
          <Search className="h-5 w-5" />
        </Button>

        {/* Search */}
        <button
          type="button"
          onClick={onCommandPaletteOpen}
          className="hidden items-center gap-2 rounded-lg border border-border bg-muted/50 px-3 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted md:flex"
        >
          <Search className="h-4 w-4" />
          <span>Search events...</span>
          <kbd className="pointer-events-none ml-8 inline-flex h-5 select-none items-center gap-1 rounded border border-border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
            <span className="text-xs">&#8984;</span>K
          </kbd>
        </button>
      </div>

      {/* Right section */}
      <div className="flex items-center gap-2">
        {/* Time Travel Picker */}
        <TimeTravelPicker />

        {/* Theme toggle */}
        <Button
          variant="ghost"
          size="icon"
          onClick={() => setTheme(theme === "dark" ? "light" : "dark")}
          aria-label="Toggle theme"
        >
          <Sun className="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
          <Moon className="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
        </Button>

        {/* User menu */}
        <div className="relative">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setShowUserMenu(!showUserMenu)}
            className="rounded-full"
            aria-label="User menu"
            aria-expanded={showUserMenu}
          >
            {user?.avatar_url ? (
              <Image
                src={user.avatar_url}
                alt={user.name}
                width={32}
                height={32}
                unoptimized
                className="h-8 w-8 rounded-full"
              />
            ) : (
              <div className="flex h-8 w-8 items-center justify-center rounded-full bg-primary/10 text-sm font-medium text-primary">
                {user?.name?.charAt(0).toUpperCase() || "U"}
              </div>
            )}
          </Button>

          {/* Dropdown menu */}
          {showUserMenu && (
            <>
              <button
                type="button"
                className="fixed inset-0 z-40 cursor-default"
                onClick={() => setShowUserMenu(false)}
                aria-label="Close user menu"
              />
              <div className="absolute right-0 top-full z-50 mt-2 w-56 rounded-lg border border-border bg-background p-1 shadow-lg">
                <div className="border-b border-border px-3 py-2">
                  <p className="text-sm font-medium">{user?.name}</p>
                  <p className="text-xs text-muted-foreground">{user?.email}</p>
                </div>
                <div className="py-1">
                  <button
                    type="button"
                    onClick={() => {
                      setShowUserMenu(false);
                      router.push("/dashboard/settings");
                    }}
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted"
                  >
                    <Settings className="h-4 w-4" />
                    Settings
                  </button>
                </div>
                <div className="border-t border-border py-1">
                  <button
                    type="button"
                    onClick={handleLogout}
                    className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-destructive hover:bg-destructive/10"
                  >
                    <LogOut className="h-4 w-4" />
                    Log out
                  </button>
                </div>
              </div>
            </>
          )}
        </div>
      </div>
    </header>
  );
}
