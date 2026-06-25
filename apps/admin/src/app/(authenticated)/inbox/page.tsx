"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { AlertTriangle, Mail } from "lucide-react";
import { useRouter, useSearchParams } from "next/navigation";
import { useCallback, useEffect, useState } from "react";
import { ConnectDialog } from "@/components/inbox/connect-dialog";
import { ConnectionsTable } from "@/components/inbox/connections-table";
import { MessageStream } from "@/components/inbox/message-stream";
import { Toaster, useToasts } from "@/components/inbox/toast";
import {
  ApiError,
  type DraftRequest,
  type DraftResponse,
  disconnect,
  draft,
  type EmailThread,
  fetchConnections,
  fetchMessages,
  groupIntoThreads,
  type InboxConnection,
  type SendRequest,
  send,
  type TriageLabel,
  triage,
} from "@/lib/inbox-api";

const MESSAGE_LIMIT = 100;

export default function InboxPage() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const { toasts, toastSuccess, toastError, dismiss } = useToasts();

  const [connections, setConnections] = useState<InboxConnection[]>([]);
  const [connectionsLoading, setConnectionsLoading] = useState(true);
  // null = not yet attempted; non-null = the last fatal load error to surface
  // (auth failures must show, never silently render an empty page — MEMORY).
  const [loadError, setLoadError] = useState<string | null>(null);

  const [selected, setSelected] = useState<InboxConnection | null>(null);
  const [threads, setThreads] = useState<EmailThread[]>([]);
  const [messagesLoading, setMessagesLoading] = useState(false);

  const [connectOpen, setConnectOpen] = useState(false);

  // ── Connections ───────────────────────────────────────────────────────
  const loadConnections = useCallback(async (): Promise<InboxConnection[]> => {
    setConnectionsLoading(true);
    setLoadError(null);
    try {
      const conns = await fetchConnections();
      setConnections(conns);
      return conns;
    } catch (err) {
      const status = err instanceof ApiError ? err.status : 0;
      if (status === 401 || status === 403) {
        setLoadError(
          "Your admin session isn't authorized for the Control Plane (401/403). Try signing in again."
        );
      } else if (status === 503) {
        setLoadError(
          "The inbox is not configured on the Control Plane (503). Set the Nylas + connector secrets and redeploy."
        );
      } else {
        setLoadError(err instanceof Error ? err.message : "Failed to load mailbox connections.");
      }
      setConnections([]);
      return [];
    } finally {
      setConnectionsLoading(false);
    }
  }, []);

  // ── Messages for the selected connection ───────────────────────────────
  const loadMessages = useCallback(
    async (conn: InboxConnection) => {
      setMessagesLoading(true);
      try {
        const events = await fetchMessages({ tenant_id: conn.tenant_id, limit: MESSAGE_LIMIT });
        setThreads(groupIntoThreads(events));
      } catch (err) {
        setThreads([]);
        toastError(err instanceof Error ? err.message : "Failed to load the email stream.");
      } finally {
        setMessagesLoading(false);
      }
    },
    [toastError]
  );

  const handleSelect = useCallback(
    (conn: InboxConnection) => {
      setSelected(conn);
      void loadMessages(conn);
    },
    [loadMessages]
  );

  // ── OAuth round-trip: finish the connect flow on mount via ?status= ────
  // The CP callback bounces back to /inbox?status=connected&email=… (or
  // ?status=error&reason=…). Toast the outcome, refresh the list, then strip the
  // query so a refresh doesn't re-toast.
  // biome-ignore lint/correctness/useExhaustiveDependencies: run only when the connect query params change; the toast/load helpers are stable.
  useEffect(() => {
    const status = searchParams.get("status");
    if (!status) return;

    if (status === "connected") {
      const email = searchParams.get("email");
      toastSuccess(email ? `Connected ${email}.` : "Mailbox connected.");
      void loadConnections();
    } else if (status === "error") {
      const reason = searchParams.get("reason");
      toastError(reason ? `Connect failed: ${reason}` : "Connecting the mailbox failed.");
    }
    // Strip the connect params from the URL (keep the path).
    router.replace("/inbox");
  }, [searchParams]);

  // Initial load.
  useEffect(() => {
    void loadConnections();
  }, [loadConnections]);

  // ── Disconnect (optimistic remove + toast) ─────────────────────────────
  const handleDisconnect = useCallback(
    async (conn: InboxConnection) => {
      const prev = connections;
      // Optimistic remove.
      setConnections((cs) => cs.filter((c) => c.grant_id !== conn.grant_id));
      if (selected?.grant_id === conn.grant_id) {
        setSelected(null);
        setThreads([]);
      }
      try {
        await disconnect(conn.grant_id);
        toastSuccess(`Disconnected ${conn.email}.`);
      } catch (err) {
        // Roll back the optimistic remove on failure.
        setConnections(prev);
        toastError(err instanceof Error ? err.message : "Failed to disconnect.");
        throw err; // let the dialog surface it too
      }
    },
    [connections, selected, toastSuccess, toastError]
  );

  // ── Triage ──────────────────────────────────────────────────────────────
  const handleTriage = useCallback(
    async (messageId: string, threadId: string, label: TriageLabel) => {
      if (!selected) return;
      try {
        await triage({
          tenant_id: selected.tenant_id,
          message_id: messageId,
          thread_id: threadId,
          label,
          by: "human",
        });
        toastSuccess(`Triaged as ${label}.`);
        await loadMessages(selected);
      } catch (err) {
        toastError(err instanceof Error ? err.message : "Failed to triage.");
        throw err;
      }
    },
    [selected, loadMessages, toastSuccess, toastError]
  );

  // ── Draft (composer passes tenant_id:"" — fill it from the selection) ──
  const handleDraft = useCallback(
    async (req: DraftRequest): Promise<DraftResponse> => {
      if (!selected) throw new Error("No mailbox selected.");
      try {
        const res = await draft({ ...req, tenant_id: selected.tenant_id, by: "human" });
        toastSuccess(`Draft saved (${res.draft_id}).`);
        await loadMessages(selected);
        return res;
      } catch (err) {
        toastError(err instanceof Error ? err.message : "Failed to save draft.");
        throw err;
      }
    },
    [selected, loadMessages, toastSuccess, toastError]
  );

  // ── Send (confirm-gated in the dialog; here we fill tenant + refresh) ──
  const handleSend = useCallback(
    async (req: SendRequest) => {
      if (!selected) throw new Error("No mailbox selected.");
      try {
        const res = await send({ ...req, tenant_id: selected.tenant_id });
        toastSuccess(res.warning ? `Sent, but ${res.warning}.` : `Sent (${res.message_id}).`);
        await loadMessages(selected);
      } catch (err) {
        toastError(err instanceof Error ? err.message : "Send failed.");
        throw err;
      }
    },
    [selected, loadMessages, toastSuccess, toastError]
  );

  // The /inbox page must be an absolute URL for the CP return_to allowlist.
  const returnTo = typeof window !== "undefined" ? `${window.location.origin}/inbox` : "";

  return (
    <div className="space-y-6" data-testid="inbox-page">
      <div>
        <h1 className="flex items-center gap-2 text-3xl font-bold tracking-tight">
          <Mail className="h-7 w-7" />
          Inbox
        </h1>
        <p className="text-muted-foreground">
          Connect tenant mailboxes, view the email stream, and triage, draft, and send — all from
          here.
        </p>
      </div>

      {/* Fatal load error (auth / not-configured) — surfaced, never swallowed. */}
      {loadError && (
        <Card className="border-red-500/30 bg-red-500/5" data-testid="inbox-load-error">
          <CardContent className="flex items-start gap-3 py-4">
            <AlertTriangle className="mt-0.5 h-5 w-5 shrink-0 text-red-500" />
            <div className="flex-1 space-y-2">
              <p className="text-sm font-medium text-red-500">Couldn&rsquo;t load the inbox</p>
              <p className="text-sm text-muted-foreground">{loadError}</p>
              <Button
                variant="outline"
                size="sm"
                onClick={() => void loadConnections()}
                data-testid="inbox-retry-btn"
              >
                Retry
              </Button>
            </div>
          </CardContent>
        </Card>
      )}

      <ConnectionsTable
        connections={connections}
        isLoading={connectionsLoading}
        selectedGrantId={selected?.grant_id}
        onSelect={handleSelect}
        onConnect={() => setConnectOpen(true)}
        onDisconnect={handleDisconnect}
      />

      <MessageStream
        connectionEmail={selected?.email}
        threads={threads}
        isLoading={messagesLoading}
        hasSelection={selected !== null}
        onTriage={handleTriage}
        onDraft={handleDraft}
        onSend={handleSend}
      />

      <ConnectDialog
        open={connectOpen}
        onClose={() => setConnectOpen(false)}
        returnTo={returnTo}
        defaultTenantId={selected?.tenant_id}
      />

      <Toaster toasts={toasts} onDismiss={dismiss} />
    </div>
  );
}
