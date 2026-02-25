"use client";

import { cn } from "@allsource/ui/utils";
import { FlaskConical } from "lucide-react";
import { useAuthStore } from "@/lib/stores/auth-store";

export function DemoBanner() {
  const tenant = useAuthStore((s) => s.tenant);

  if (!tenant?.is_demo) return null;

  return (
    <div
      className={cn(
        "flex items-center justify-center gap-2 px-4 py-2",
        "bg-gradient-to-r from-amber-500/10 via-amber-500/5 to-amber-500/10",
        "border-b border-amber-500/20 text-sm"
      )}
    >
      <FlaskConical className="h-4 w-4 text-amber-600 dark:text-amber-400" />
      <span className="text-muted-foreground">
        <span className="font-medium text-foreground">Demo Account</span>
        {" — "}
        You&apos;re using a demo account. Data resets daily.
      </span>
      <a
        href="/signup"
        className="ml-2 font-medium text-primary hover:underline"
      >
        Create a real account
      </a>
    </div>
  );
}
