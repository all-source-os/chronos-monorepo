"use client";

import { Button } from "@allsource/ui";
import { Eye } from "lucide-react";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { startViewAs } from "@/lib/viewas-api";

/**
 * "View as tenant" entry point (ADMIN_TENANT_POWER_TOOL §5.3) for the tenant 360.
 *
 * Clicking it mints the read-only view-as token SERVER-SIDE (POST
 * /api/viewas/start/:id sets the separate viewas_token cookie; the raw token never
 * reaches client JS), then drops the operator into the read-only product frame
 * (/view-as/:id) where the persistent banner + countdown + Exit live.
 *
 * Read-only is enforced server-side (readonly role + the CP view_as write-refusal
 * + the read-only data proxy) — this button just starts the session. On a failed
 * mint (e.g. 404 unknown tenant, 503 not configured) it surfaces the real error
 * inline rather than navigating into a dead frame.
 */

interface ViewAsButtonProps {
  tenantId: string;
}

export function ViewAsButton({ tenantId }: ViewAsButtonProps) {
  const router = useRouter();
  const [starting, setStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const handleStart = async () => {
    setStarting(true);
    setError(null);
    try {
      await startViewAs(tenantId);
      // The viewas_token cookie is now set server-side; enter the read-only frame.
      router.push(`/view-as/${tenantId}`);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start view-as.");
      setStarting(false);
    }
  };

  return (
    <div className="flex flex-col items-end gap-1">
      <Button
        variant="outline"
        onClick={handleStart}
        disabled={starting}
        data-testid="viewas-start-btn"
      >
        <Eye className="mr-2 h-4 w-4" />
        {starting ? "Starting…" : "View as tenant"}
      </Button>
      {error && (
        <p
          className="max-w-xs text-right text-xs text-destructive"
          data-testid="viewas-start-error"
        >
          {error}
        </p>
      )}
    </div>
  );
}
