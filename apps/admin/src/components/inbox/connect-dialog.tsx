"use client";

import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@allsource/ui";
import { ExternalLink, Mailbox } from "lucide-react";
import { useState } from "react";
import { type ConnectResponse, startConnect } from "@/lib/inbox-api";
import { TenantSelect } from "./tenant-select";

/**
 * Connect-mailbox dialog. Collects tenant_id + optional email, calls the CP's
 * hosted-OAuth Start endpoint, then sends the WHOLE browser to the returned
 * auth_url (a full-page redirect, not a popup — Nylas consent doesn't survive a
 * popup blocker, and a full redirect is the simplest way to land back on /inbox
 * via the CP callback's `return_to?status=…` bounce). Mirrors the Card-as-modal
 * pattern of suspend-dialog.tsx / edit-quotas-dialog.tsx.
 *
 * `returnTo` is the absolute /inbox URL the CP will redirect back to; the CP only
 * honours it if its origin is on the ADMIN_DASHBOARD_ORIGIN allowlist.
 */
interface ConnectDialogProps {
  open: boolean;
  onClose: () => void;
  /** Absolute URL of this /inbox page (passed as return_to). */
  returnTo: string;
  /** Pre-fill the tenant id (e.g. the currently selected connection's tenant). */
  defaultTenantId?: string;
}

export function ConnectDialog({ open, onClose, returnTo, defaultTenantId }: ConnectDialogProps) {
  const [tenantId, setTenantId] = useState(defaultTenantId ?? "");
  const [email, setEmail] = useState("");
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleStart = async () => {
    if (tenantId.trim() === "") {
      setError("A tenant id is required.");
      return;
    }
    setIsStarting(true);
    setError(null);
    try {
      const res: ConnectResponse = await startConnect({
        tenant_id: tenantId.trim(),
        email: email.trim() || undefined,
        return_to: returnTo,
      });
      if (!res.auth_url) {
        setError("The Control Plane did not return an auth URL.");
        setIsStarting(false);
        return;
      }
      // Leave the app — the CP callback bounces back to returnTo?status=…
      window.location.href = res.auth_url;
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start the connect flow.");
      setIsStarting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="inbox-connect-dialog"
    >
      {/* Backdrop (click to dismiss — mirrors SuspendDialog) */}
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mailbox className="h-5 w-5" />
            Connect a mailbox
          </CardTitle>
          <CardDescription>
            Start the hosted OAuth flow to connect a tenant&rsquo;s mailbox. You&rsquo;ll be sent to
            the provider to grant access, then returned here.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Tenant</Label>
            <TenantSelect value={tenantId} onChange={(id) => setTenantId(id)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="connect-email">Mailbox email (optional)</Label>
            <Input
              id="connect-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="sales@all-source.xyz"
              autoComplete="off"
              data-testid="connect-email-input"
            />
            <p className="text-xs text-muted-foreground">
              Pre-fills the address on the provider&rsquo;s consent screen.
            </p>
          </div>

          {error && (
            <p className="text-sm text-red-500" data-testid="connect-error">
              {error}
            </p>
          )}
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button
            variant="outline"
            onClick={onClose}
            disabled={isStarting}
            data-testid="connect-cancel-btn"
          >
            Cancel
          </Button>
          <Button onClick={handleStart} disabled={isStarting} data-testid="connect-start-btn">
            <ExternalLink className="mr-1.5 h-4 w-4" />
            {isStarting ? "Redirecting…" : "Continue to provider"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
