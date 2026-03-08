"use client";

import { Button, Card, CardContent, CardFooter, CardHeader, CardTitle, Input, Label } from "@allsource/ui";
import { useState } from "react";
import type { TenantQuotas } from "@/lib/tenants-api";

interface EditQuotasDialogProps {
  open: boolean;
  onClose: () => void;
  quotas: TenantQuotas;
  onSave: (quotas: TenantQuotas) => Promise<void>;
}

export function EditQuotasDialog({ open, onClose, quotas, onSave }: EditQuotasDialogProps) {
  const [eventLimit, setEventLimit] = useState(String(quotas.event_limit));
  const [queryLimit, setQueryLimit] = useState(String(quotas.query_limit));
  const [storageLimitMb, setStorageLimitMb] = useState(String(quotas.storage_limit_mb));
  const [isSaving, setIsSaving] = useState(false);

  if (!open) return null;

  const handleSave = async () => {
    setIsSaving(true);
    try {
      await onSave({
        event_limit: Number(eventLimit),
        query_limit: Number(queryLimit),
        storage_limit_mb: Number(storageLimitMb),
      });
      onClose();
    } catch (err) {
      console.error("Failed to save quotas:", err);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center" data-testid="edit-quotas-dialog">
      {/* Backdrop */}
      <div className="fixed inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      {/* Dialog */}
      <Card className="relative z-50 w-full max-w-md mx-4">
        <CardHeader>
          <CardTitle>Edit Quotas</CardTitle>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="event_limit">Event Limit</Label>
            <Input
              id="event_limit"
              type="number"
              value={eventLimit}
              onChange={(e) => setEventLimit(e.target.value)}
              data-testid="quota-event-limit"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="query_limit">Query Limit</Label>
            <Input
              id="query_limit"
              type="number"
              value={queryLimit}
              onChange={(e) => setQueryLimit(e.target.value)}
              data-testid="quota-query-limit"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor="storage_limit_mb">Storage Limit (MB)</Label>
            <Input
              id="storage_limit_mb"
              type="number"
              value={storageLimitMb}
              onChange={(e) => setStorageLimitMb(e.target.value)}
              data-testid="quota-storage-limit"
            />
          </div>
        </CardContent>
        <CardFooter className="flex justify-end gap-2">
          <Button variant="outline" onClick={onClose} data-testid="quota-cancel-btn">
            Cancel
          </Button>
          <Button onClick={handleSave} disabled={isSaving} data-testid="quota-save-btn">
            {isSaving ? "Saving..." : "Save"}
          </Button>
        </CardFooter>
      </Card>
    </div>
  );
}
