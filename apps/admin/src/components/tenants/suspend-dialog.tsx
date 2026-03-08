"use client";

import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle } from "@allsource/ui";
import { useState } from "react";

interface SuspendDialogProps {
  open: boolean;
  onClose: () => void;
  tenantName: string;
  isSuspended: boolean;
  onConfirm: () => Promise<void>;
}

export function SuspendDialog({ open, onClose, tenantName, isSuspended, onConfirm }: SuspendDialogProps) {
  const [isSubmitting, setIsSubmitting] = useState(false);

  if (!open) return null;

  const handleConfirm = async () => {
    setIsSubmitting(true);
    try {
      await onConfirm();
      onClose();
    } catch (err) {
      console.error("Failed to toggle suspension:", err);
    } finally {
      setIsSubmitting(false);
    }
  };

  const action = isSuspended ? "Unsuspend" : "Suspend";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" data-testid="suspend-dialog">
      {/* Backdrop */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      {/* Dialog */}
      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle>{action} Tenant</CardTitle>
        </CardHeader>
        <CardContent>
          <p className="text-sm text-muted-foreground">
            {isSuspended
              ? `Are you sure you want to unsuspend "${tenantName}"? The tenant will regain access to all services.`
              : `Are you sure you want to suspend "${tenantName}"? The tenant will lose access to all services immediately.`}
          </p>
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose} data-testid="suspend-cancel-btn">
            Cancel
          </Button>
          <Button
            variant={isSuspended ? "default" : "destructive"}
            onClick={handleConfirm}
            disabled={isSubmitting}
            data-testid="suspend-confirm-btn"
          >
            {isSubmitting ? "Processing..." : action}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
