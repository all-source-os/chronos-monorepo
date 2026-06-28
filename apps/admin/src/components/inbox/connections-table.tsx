"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Skeleton,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AtSign, Inbox, Mailbox, Plus, Trash2 } from "lucide-react";
import { useState } from "react";
import { DisconnectDialog } from "@/components/inbox/disconnect-dialog";
import type { InboxConnection } from "@/lib/inbox-api";

function formatDateTime(iso?: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString("en-US", {
    year: "numeric",
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

interface ConnectionsTableProps {
  connections: InboxConnection[];
  isLoading: boolean;
  /** The grant_id of the connection whose stream is currently open. */
  selectedGrantId?: string;
  onSelect: (conn: InboxConnection) => void;
  onConnect: () => void;
  /** Open the "add hosted mailbox" (adopt existing grant) dialog. */
  onAddHosted: () => void;
  /** Open the "add receiving address" dialog (Resend — no OAuth). */
  onAddAddress: () => void;
  /** Disconnect a grant; resolves once removed (the page does the optimistic remove + toast). */
  onDisconnect: (conn: InboxConnection) => Promise<void>;
}

export function ConnectionsTable({
  connections,
  isLoading,
  selectedGrantId,
  onSelect,
  onConnect,
  onAddHosted,
  onAddAddress,
  onDisconnect,
}: ConnectionsTableProps) {
  const [pendingDisconnect, setPendingDisconnect] = useState<InboxConnection | null>(null);

  return (
    <Card data-testid="inbox-connections-card">
      <CardHeader className="flex flex-row items-start justify-between gap-4">
        <div>
          <CardTitle className="flex items-center gap-2">
            <Inbox className="h-5 w-5" />
            Connected mailboxes
          </CardTitle>
          <CardDescription>
            Mailboxes connected via hosted OAuth. Select one to view its email stream.
          </CardDescription>
        </div>
        <div className="flex shrink-0 gap-2">
          <Button variant="outline" onClick={onAddHosted} data-testid="inbox-add-hosted-btn">
            <Mailbox className="mr-1.5 h-4 w-4" />
            Add hosted mailbox
          </Button>
          <Button variant="outline" onClick={onConnect} data-testid="inbox-connect-btn">
            <Plus className="mr-1.5 h-4 w-4" />
            Connect (OAuth)
          </Button>
          <Button onClick={onAddAddress} data-testid="inbox-add-address-btn">
            <AtSign className="mr-1.5 h-4 w-4" />
            Add address
          </Button>
        </div>
      </CardHeader>
      <CardContent>
        {isLoading ? (
          <div className="space-y-3" data-testid="inbox-connections-skeleton">
            {Array.from({ length: 3 }).map((_, i) => (
              <Skeleton key={`conn-skeleton-${i}`} className="h-12 w-full" />
            ))}
          </div>
        ) : connections.length === 0 ? (
          <div
            className="py-12 text-center text-sm text-muted-foreground"
            data-testid="inbox-connections-empty"
          >
            No mailboxes connected yet. Click <span className="font-medium">Connect mailbox</span>{" "}
            to start.
          </div>
        ) : (
          <div data-testid="inbox-connections-table">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Email</TableHead>
                  <TableHead>Tenant</TableHead>
                  <TableHead>Provider</TableHead>
                  <TableHead>Connected</TableHead>
                  <TableHead className="text-right">Actions</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {connections.map((conn) => {
                  const isSelected = conn.grant_id === selectedGrantId;
                  return (
                    <TableRow
                      key={conn.grant_id}
                      className={cn("cursor-pointer", isSelected && "bg-primary/5")}
                      onClick={() => onSelect(conn)}
                      data-testid={`inbox-connection-row-${conn.grant_id}`}
                    >
                      <TableCell className="font-medium">
                        <div className="flex items-center gap-2">
                          {conn.email}
                          {isSelected && (
                            <Badge variant="secondary" className="text-[10px]">
                              Viewing
                            </Badge>
                          )}
                        </div>
                      </TableCell>
                      <TableCell className="font-mono text-xs text-muted-foreground">
                        {conn.tenant_id}
                      </TableCell>
                      <TableCell>
                        <Badge variant="outline" className="capitalize">
                          {conn.provider}
                        </Badge>
                      </TableCell>
                      <TableCell>{formatDateTime(conn.connected_at)}</TableCell>
                      <TableCell className="text-right">
                        <Button
                          variant="ghost"
                          size="icon"
                          aria-label={`Disconnect ${conn.email}`}
                          onClick={(e) => {
                            e.stopPropagation();
                            setPendingDisconnect(conn);
                          }}
                          data-testid={`inbox-disconnect-btn-${conn.grant_id}`}
                        >
                          <Trash2 className="h-4 w-4 text-red-500" />
                        </Button>
                      </TableCell>
                    </TableRow>
                  );
                })}
              </TableBody>
            </Table>
          </div>
        )}
      </CardContent>

      <DisconnectDialog
        open={pendingDisconnect !== null}
        email={pendingDisconnect?.email ?? ""}
        onClose={() => setPendingDisconnect(null)}
        onConfirm={async () => {
          if (pendingDisconnect) await onDisconnect(pendingDisconnect);
        }}
      />
    </Card>
  );
}
