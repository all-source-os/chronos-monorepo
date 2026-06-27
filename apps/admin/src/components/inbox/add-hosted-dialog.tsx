"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Label,
  Skeleton,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Check, Mailbox, RefreshCw } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { TenantSelect } from "@/components/inbox/tenant-select";
import { type AvailableGrant, adoptGrant, availableGrants } from "@/lib/inbox-api";

interface AddHostedDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called after a successful adopt — the parent refreshes connections. */
  onAdopted: () => void;
}

/**
 * Add a hosted/existing provider mailbox (e.g. a Nylas-hosted address like
 * sales@all-source.xyz) to a tenant — no OAuth login. Lists the provider's
 * grants, lets the operator pick an unregistered one + a tenant, and POSTs the
 * adopt. The mailbox must already exist in the provider (Nylas dashboard).
 */
export function AddHostedDialog({ open, onClose, onAdopted }: AddHostedDialogProps) {
  const [tenantId, setTenantId] = useState("");
  const [grants, setGrants] = useState<AvailableGrant[]>([]);
  const [selectedGrant, setSelectedGrant] = useState("");
  const [loading, setLoading] = useState(false);
  const [adopting, setAdopting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = useCallback(async () => {
    setLoading(true);
    setError(null);
    try {
      setGrants(await availableGrants());
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to list provider mailboxes.");
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) {
      setTenantId("");
      setSelectedGrant("");
      setError(null);
      void load();
    }
  }, [open, load]);

  if (!open) return null;

  const handleAdopt = async () => {
    if (tenantId === "") {
      setError("Pick a tenant.");
      return;
    }
    if (selectedGrant === "") {
      setError("Pick a mailbox.");
      return;
    }
    setAdopting(true);
    setError(null);
    try {
      await adoptGrant({ tenant_id: tenantId, grant_id: selectedGrant });
      onAdopted();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to add the mailbox.");
      setAdopting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="inbox-add-hosted-dialog"
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mailbox className="h-5 w-5" />
            Add a hosted mailbox
          </CardTitle>
          <CardDescription>
            Register a provider mailbox (e.g. a Nylas-hosted address like{" "}
            <code>sales@all-source.xyz</code>) to a tenant — no login required. It must already
            exist in the provider.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Tenant</Label>
            <TenantSelect value={tenantId} onChange={(id) => setTenantId(id)} />
          </div>

          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>Mailbox</Label>
              <Button
                variant="ghost"
                size="sm"
                onClick={() => void load()}
                disabled={loading}
                data-testid="add-hosted-refresh"
              >
                <RefreshCw className={cn("h-3.5 w-3.5", loading && "animate-spin")} />
              </Button>
            </div>
            {loading ? (
              <Skeleton className="h-24 w-full" />
            ) : grants.length === 0 ? (
              <p className="text-sm text-muted-foreground">No provider mailboxes found.</p>
            ) : (
              <div
                className="max-h-48 overflow-auto rounded-md border"
                data-testid="add-hosted-grants"
              >
                {grants.map((g) => (
                  <button
                    key={g.grant_id}
                    type="button"
                    disabled={g.registered}
                    onClick={() => setSelectedGrant(g.grant_id)}
                    className={cn(
                      "flex w-full items-center justify-between gap-2 px-3 py-2 text-left text-sm",
                      g.registered ? "cursor-not-allowed opacity-50" : "hover:bg-accent",
                      selectedGrant === g.grant_id && "bg-accent"
                    )}
                    data-testid="add-hosted-grant-option"
                  >
                    <span className="flex min-w-0 items-center gap-2">
                      {selectedGrant === g.grant_id && <Check className="h-3.5 w-3.5 shrink-0" />}
                      <span className="truncate">{g.email}</span>
                    </span>
                    {g.registered ? (
                      <Badge variant="secondary" className="shrink-0">
                        Connected
                      </Badge>
                    ) : (
                      <Badge variant="outline" className="shrink-0 capitalize">
                        {g.provider}
                      </Badge>
                    )}
                  </button>
                ))}
              </div>
            )}
          </div>

          {error && (
            <p className="text-sm text-red-500" data-testid="add-hosted-error">
              {error}
            </p>
          )}
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose} disabled={adopting}>
            Cancel
          </Button>
          <Button
            onClick={handleAdopt}
            disabled={adopting || tenantId === "" || selectedGrant === ""}
            data-testid="add-hosted-submit"
          >
            {adopting ? "Adding…" : "Add mailbox"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
