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
import { AtSign } from "lucide-react";
import { useEffect, useState } from "react";
import { TenantSelect } from "@/components/inbox/tenant-select";
import { addAddress } from "@/lib/inbox-api";

interface AddAddressDialogProps {
  open: boolean;
  onClose: () => void;
  /** Called after a successful add — the parent refreshes connections. */
  onAdded: () => void;
}

/**
 * Register a receiving address (e.g. sales@all-source.xyz) to a tenant. Used by
 * the Resend connector, where a connection is just a verified address on a domain
 * you control (no OAuth login). The domain must be verified in Resend and its MX
 * pointed at Resend for inbound to arrive.
 */
export function AddAddressDialog({ open, onClose, onAdded }: AddAddressDialogProps) {
  const [tenantId, setTenantId] = useState("");
  const [email, setEmail] = useState("");
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (open) {
      setTenantId("");
      setEmail("");
      setError(null);
    }
  }, [open]);

  if (!open) return null;

  const valid = tenantId !== "" && /^[^@\s]+@[^@\s]+\.[^@\s]+$/.test(email.trim());

  const handleAdd = async () => {
    if (!valid) {
      setError("Pick a tenant and enter a valid email address.");
      return;
    }
    setSaving(true);
    setError(null);
    try {
      await addAddress({ tenant_id: tenantId, email: email.trim() });
      onAdded();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to add the address.");
      setSaving(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="inbox-add-address-dialog"
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <AtSign className="h-5 w-5" />
            Add a receiving address
          </CardTitle>
          <CardDescription>
            Connect an address on a domain you control (e.g. <code>sales@all-source.xyz</code>) to a
            tenant. The domain must be verified in Resend with its MX pointed at Resend so inbound
            mail arrives.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label>Tenant</Label>
            <TenantSelect value={tenantId} onChange={(id) => setTenantId(id)} />
          </div>

          <div className="space-y-2">
            <Label htmlFor="add-address-email">Email address</Label>
            <Input
              id="add-address-email"
              type="email"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              placeholder="sales@all-source.xyz"
              data-testid="add-address-email"
            />
          </div>

          {error && (
            <p className="text-sm text-red-500" data-testid="add-address-error">
              {error}
            </p>
          )}
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose} disabled={saving}>
            Cancel
          </Button>
          <Button onClick={handleAdd} disabled={saving || !valid} data-testid="add-address-submit">
            {saving ? "Adding…" : "Add address"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
