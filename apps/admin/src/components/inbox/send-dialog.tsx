"use client";

import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle } from "@allsource/ui";
import { AlertTriangle, Send } from "lucide-react";
import { useState } from "react";
import { type EmailAddress, formatAddress } from "@/lib/inbox-api";

/**
 * Send confirm dialog. Sending is irreversible (mail leaves the building), so it
 * is NEVER one-click — the composer raises this dialog, which shows exactly what
 * will go out (to / subject / body preview) before the operator confirms. On
 * confirm the parent posts to /send with confirm:true. Mirrors the destructive
 * confirm UX of SuspendDialog / RecoveryDialog.
 */
interface SendDialogProps {
  open: boolean;
  onClose: () => void;
  to: EmailAddress[];
  subject?: string;
  body: string;
  /** Whether this send is tied to an existing draft (informational). */
  draftId?: string;
  /** Send for real (confirm:true). Resolves on success. */
  onConfirm: () => Promise<void>;
}

export function SendDialog({
  open,
  onClose,
  to,
  subject,
  body,
  draftId,
  onConfirm,
}: SendDialogProps) {
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
      setError(err instanceof Error ? err.message : "Send failed.");
      setIsSubmitting(false);
    }
  };

  return (
    <div
      className="fixed inset-0 z-50 flex items-center justify-center"
      data-testid="inbox-send-dialog"
    >
      {/* biome-ignore lint/a11y/noStaticElementInteractions: dismiss-on-backdrop, Cancel is the keyboard path */}
      {/* biome-ignore lint/a11y/useKeyWithClickEvents: dismiss-on-backdrop, Cancel is the keyboard path */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      <Card className="relative z-50 w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Send className="h-5 w-5" />
            Send email
          </CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div
            className="flex items-start gap-2 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3 text-sm text-amber-600 dark:text-amber-400"
            data-testid="send-warning"
          >
            <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5" />
            <p>
              This sends a real email through the connected mailbox. It can&rsquo;t be unsent.
              Confirm the recipients and contents below.
            </p>
          </div>

          <div className="space-y-2 text-sm">
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">To</span>
              {to.length === 0 ? (
                <span className="text-red-500">No recipients</span>
              ) : (
                <ul className="space-y-0.5">
                  {to.map((a) => (
                    <li key={a.email} className="font-mono text-xs">
                      {formatAddress(a)}
                    </li>
                  ))}
                </ul>
              )}
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Subject</span>
              <span>{subject?.trim() ? subject : "(no subject)"}</span>
            </div>
            <div className="flex flex-col gap-1">
              <span className="text-xs font-medium text-muted-foreground">Body</span>
              <pre className="max-h-48 overflow-auto whitespace-pre-wrap rounded-lg border bg-muted p-3 text-xs">
                {body}
              </pre>
            </div>
            {draftId && (
              <p className="text-xs text-muted-foreground">
                Linked draft: <span className="font-mono">{draftId}</span>
              </p>
            )}
          </div>

          {error && (
            <p className="text-sm text-red-500" data-testid="send-error">
              {error}
            </p>
          )}
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button
            variant="outline"
            onClick={onClose}
            disabled={isSubmitting}
            data-testid="send-cancel-btn"
          >
            Cancel
          </Button>
          <Button
            onClick={handleConfirm}
            disabled={isSubmitting || to.length === 0 || body.trim() === ""}
            data-testid="send-confirm-btn"
          >
            <Send className="mr-1.5 h-4 w-4" />
            {isSubmitting ? "Sending…" : "Send now"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
