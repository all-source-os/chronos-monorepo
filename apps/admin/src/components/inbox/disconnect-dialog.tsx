"use client";

import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle } from "@allsource/ui";
import { Trash2 } from "lucide-react";
import { useState } from "react";

/**
 * Confirm-gate for disconnecting a mailbox. Disconnect drops the per-grant record
 * from Core config (irreversible without reconnecting), so it goes behind a
 * confirm dialog — mirroring SuspendDialog's destructive-action UX.
 */
interface DisconnectDialogProps {
  open: boolean;
  onClose: () => void;
  email: string;
  onConfirm: () => Promise<void>;
}

export function DisconnectDialog({ open, onClose, email, onConfirm }: DisconnectDialogProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  const handleConfirm = async () => {
    setIsSubmitting(true);
    setError(null);
    try {
      await onConfirm();
      onClose();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to disconnect.");
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="inbox-disconnect-dialog"
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Trash2 className="h-5 w-5 text-red-500" />
            Disconnect mailbox
          </CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            Are you sure you want to disconnect{" "}
            <span className="font-medium text-foreground">{email}</span>? The grant will be removed
            and the inbox will stop syncing. You&rsquo;ll need to run the connect flow again to
            reconnect it.
          </p>
          {error && (
            <p className="mt-3 text-sm text-red-500" data-testid="disconnect-error">
              {error}
            </p>
          )}
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button
            variant="outline"
            onClick={onClose}
            disabled={isSubmitting}
            data-testid="disconnect-cancel-btn"
          >
            Cancel
          </Button>
          <Button
            variant="destructive"
            onClick={handleConfirm}
            disabled={isSubmitting}
            data-testid="disconnect-confirm-btn"
          >
            {isSubmitting ? "Disconnecting…" : "Disconnect"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
