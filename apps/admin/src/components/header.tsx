"use client";

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { LogOut, Menu, ShieldCheck } from "lucide-react";
import { useRouter } from "next/navigation";

interface HeaderProps {
  sidebarCollapsed: boolean;
  onMenuClick: () => void;
}

export function Header({ sidebarCollapsed, onMenuClick }: HeaderProps) {
  const router = useRouter();

  const handleLogout = async () => {
    await fetch("/api/auth/session", { method: "DELETE" });
    router.push("/login");
  };

  return (
    <header
      className={cn(
        "fixed top-0 right-0 z-30 flex h-16 items-center justify-between border-b border-border bg-background/95 px-4 backdrop-blur-sm transition-all duration-300",
        sidebarCollapsed ? "left-16" : "left-64",
        "max-md:left-0"
      )}
    >
      {/* Mobile menu button */}
      <Button
        variant="ghost"
        size="icon"
        className="md:hidden"
        onClick={onMenuClick}
        aria-label="Open menu"
      >
        <Menu className="h-5 w-5" />
      </Button>

      {/* Admin badge */}
      <div className="hidden md:flex items-center gap-2 text-sm text-muted-foreground">
        <ShieldCheck className="h-4 w-4" />
        <span>Administration</span>
      </div>

      {/* Right side */}
      <Button
        variant="ghost"
        size="sm"
        onClick={handleLogout}
        className="text-muted-foreground hover:text-foreground"
      >
        <LogOut className="mr-2 h-4 w-4" />
        Sign out
      </Button>
    </header>
  );
}
