"use client";

import { useState } from "react";
import {
  Drawer,
  DrawerContent,
  DrawerHeader,
  DrawerTitle,
  DrawerDescription,
  DrawerFooter,
  Button,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Copy, Check, Clock, Hash, Layers, ExternalLink } from "lucide-react";
import type { Event } from "@/lib/api/client";

interface EventDetailDrawerProps {
  event: Event | null;
  open: boolean;
  onClose: () => void;
  onViewEntity?: (entityId: string) => void;
}

export function EventDetailDrawer({
  event,
  open,
  onClose,
  onViewEntity,
}: EventDetailDrawerProps) {
  const [copiedField, setCopiedField] = useState<string | null>(null);

  if (!event) return null;

  const copyToClipboard = async (text: string, field: string) => {
    await navigator.clipboard.writeText(text);
    setCopiedField(field);
    setTimeout(() => setCopiedField(null), 2000);
  };

  const getEventTypeColor = (eventType: string) => {
    if (eventType.includes("user")) return "bg-blue-500/10 text-blue-600 dark:text-blue-400";
    if (eventType.includes("order")) return "bg-green-500/10 text-green-600 dark:text-green-400";
    if (eventType.includes("payment")) return "bg-purple-500/10 text-purple-600 dark:text-purple-400";
    if (eventType.includes("inventory")) return "bg-orange-500/10 text-orange-600 dark:text-orange-400";
    return "bg-muted text-muted-foreground";
  };

  const formatTimestamp = (timestamp: string) => {
    const date = new Date(timestamp);
    return {
      full: date.toISOString(),
      display: date.toLocaleString(),
      relative: getRelativeTime(date),
    };
  };

  const getRelativeTime = (date: Date) => {
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffSec = Math.floor(diffMs / 1000);
    const diffMin = Math.floor(diffSec / 60);
    const diffHour = Math.floor(diffMin / 60);
    const diffDay = Math.floor(diffHour / 24);

    if (diffSec < 60) return `${diffSec} seconds ago`;
    if (diffMin < 60) return `${diffMin} minutes ago`;
    if (diffHour < 24) return `${diffHour} hours ago`;
    return `${diffDay} days ago`;
  };

  const { full, display, relative } = formatTimestamp(event.timestamp);

  return (
    <Drawer open={open} onOpenChange={(isOpen) => !isOpen && onClose()}>
      <DrawerContent>
        <div className="mx-auto w-full max-w-2xl">
          <DrawerHeader>
            <div className="flex items-center gap-3">
              <span
                className={cn(
                  "rounded-full px-3 py-1 text-sm font-medium",
                  getEventTypeColor(event.event_type)
                )}
              >
                {event.event_type}
              </span>
            </div>
            <DrawerTitle className="mt-2 text-xl">{event.entity_id}</DrawerTitle>
            <DrawerDescription className="flex items-center gap-1">
              <Clock className="h-3 w-3" />
              {relative}
            </DrawerDescription>
          </DrawerHeader>

          <div className="px-4 pb-4">
            {/* Metadata grid */}
            <div className="grid gap-4 sm:grid-cols-2">
              {/* Event ID */}
              <div className="rounded-lg border border-border p-3">
                <div className="mb-1 flex items-center justify-between">
                  <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Hash className="h-3 w-3" />
                    Event ID
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={() => copyToClipboard(event.id, "id")}
                  >
                    {copiedField === "id" ? (
                      <Check className="h-3 w-3 text-green-500" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
                <p className="truncate font-mono text-sm">{event.id}</p>
              </div>

              {/* Entity ID */}
              <div className="rounded-lg border border-border p-3">
                <div className="mb-1 flex items-center justify-between">
                  <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Layers className="h-3 w-3" />
                    Entity ID
                  </span>
                  <div className="flex items-center gap-1">
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={() => onViewEntity?.(event.entity_id)}
                    >
                      <ExternalLink className="h-3 w-3" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-6 w-6"
                      onClick={() => copyToClipboard(event.entity_id, "entity")}
                    >
                      {copiedField === "entity" ? (
                        <Check className="h-3 w-3 text-green-500" />
                      ) : (
                        <Copy className="h-3 w-3" />
                      )}
                    </Button>
                  </div>
                </div>
                <p className="truncate font-mono text-sm">{event.entity_id}</p>
              </div>

              {/* Version */}
              <div className="rounded-lg border border-border p-3">
                <span className="text-xs text-muted-foreground">Version</span>
                <p className="font-mono text-sm">{event.version}</p>
              </div>

              {/* Timestamp */}
              <div className="rounded-lg border border-border p-3">
                <div className="mb-1 flex items-center justify-between">
                  <span className="flex items-center gap-1.5 text-xs text-muted-foreground">
                    <Clock className="h-3 w-3" />
                    Timestamp
                  </span>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-6 w-6"
                    onClick={() => copyToClipboard(full, "timestamp")}
                  >
                    {copiedField === "timestamp" ? (
                      <Check className="h-3 w-3 text-green-500" />
                    ) : (
                      <Copy className="h-3 w-3" />
                    )}
                  </Button>
                </div>
                <p className="text-sm">{display}</p>
                <p className="font-mono text-xs text-muted-foreground">{full}</p>
              </div>
            </div>

            {/* Payload */}
            <div className="mt-4 rounded-lg border border-border p-4">
              <div className="mb-2 flex items-center justify-between">
                <span className="text-sm font-medium">Payload</span>
                <Button
                  variant="ghost"
                  size="sm"
                  onClick={() =>
                    copyToClipboard(
                      JSON.stringify(event.payload, null, 2),
                      "payload"
                    )
                  }
                >
                  {copiedField === "payload" ? (
                    <>
                      <Check className="mr-1.5 h-3 w-3 text-green-500" />
                      Copied
                    </>
                  ) : (
                    <>
                      <Copy className="mr-1.5 h-3 w-3" />
                      Copy JSON
                    </>
                  )}
                </Button>
              </div>
              <pre className="overflow-x-auto rounded-lg bg-muted p-3 text-xs">
                <code>{JSON.stringify(event.payload, null, 2)}</code>
              </pre>
            </div>
          </div>

          <DrawerFooter>
            <Button variant="outline" onClick={onClose}>
              Close
            </Button>
          </DrawerFooter>
        </div>
      </DrawerContent>
    </Drawer>
  );
}
