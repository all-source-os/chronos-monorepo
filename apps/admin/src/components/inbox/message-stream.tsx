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
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  ArrowDownLeft,
  ArrowUpRight,
  ChevronDown,
  ChevronRight,
  FileText,
  Mail,
  Reply,
} from "lucide-react";
import { useState } from "react";
import { DraftComposer } from "@/components/inbox/draft-composer";
import { TriageControl } from "@/components/inbox/triage-control";
import {
  type DraftRequest,
  type DraftResponse,
  type EmailMessageView,
  type EmailThread,
  formatAddress,
  type SendRequest,
  type TriageLabel,
} from "@/lib/inbox-api";

function formatDateTime(iso?: string): string {
  if (!iso) return "—";
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return "—";
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function DirectionIcon({ message }: { message: EmailMessageView }) {
  if (message.type === "drafted") {
    return <FileText className="h-4 w-4 text-muted-foreground" aria-label="draft" />;
  }
  if (message.direction === "outbound") {
    return <ArrowUpRight className="h-4 w-4 text-emerald-500" aria-label="outbound" />;
  }
  return <ArrowDownLeft className="h-4 w-4 text-blue-500" aria-label="inbound" />;
}

function MessageRow({ message }: { message: EmailMessageView }) {
  const isDraft = message.type === "drafted";
  return (
    <div
      className={cn(
        "flex items-start gap-3 rounded-lg border p-3",
        isDraft && "border-dashed bg-muted/40"
      )}
      data-testid={`inbox-message-${message.id}`}
    >
      <DirectionIcon message={message} />
      <div className="min-w-0 flex-1 space-y-1">
        <div className="flex items-center justify-between gap-2">
          <span className="truncate text-sm font-medium">
            {message.type === "received"
              ? formatAddress(message.from)
              : message.to.length > 0
                ? `To: ${message.to.map((a) => a.email).join(", ")}`
                : isDraft
                  ? "Draft"
                  : "Outbound"}
          </span>
          <span className="shrink-0 text-xs text-muted-foreground">
            {formatDateTime(message.timestamp)}
          </span>
        </div>
        {(message.snippet || message.body) && (
          <p className="line-clamp-3 whitespace-pre-wrap text-sm text-muted-foreground">
            {message.snippet || message.body}
          </p>
        )}
        {isDraft && message.intent && (
          <Badge variant="outline" className="text-[10px]">
            intent: {message.intent}
          </Badge>
        )}
      </div>
    </div>
  );
}

interface ThreadCardProps {
  thread: EmailThread;
  onTriage: (messageId: string, threadId: string, label: TriageLabel) => Promise<void>;
  onDraft: (req: DraftRequest) => Promise<DraftResponse>;
  onSend: (req: SendRequest) => Promise<void>;
}

function ThreadCard({ thread, onTriage, onDraft, onSend }: ThreadCardProps) {
  const [expanded, setExpanded] = useState(false);
  const [replyOpen, setReplyOpen] = useState(false);

  // Anchor triage + reply on the thread's latest inbound message when present,
  // else its latest message (drafts/sent share the message_id contract).
  const inbound = [...thread.messages].reverse().find((m) => m.type === "received");
  const latest = thread.messages.at(-1);
  const anchor = inbound ?? latest;

  return (
    <Card data-testid={`inbox-thread-${thread.threadId}`}>
      <CardHeader className="space-y-2 pb-3">
        <div className="flex items-start justify-between gap-3">
          <button
            type="button"
            className="flex min-w-0 flex-1 items-start gap-2 text-left"
            onClick={() => setExpanded((v) => !v)}
            data-testid={`inbox-thread-toggle-${thread.threadId}`}
          >
            {expanded ? (
              <ChevronDown className="mt-1 h-4 w-4 shrink-0 text-muted-foreground" />
            ) : (
              <ChevronRight className="mt-1 h-4 w-4 shrink-0 text-muted-foreground" />
            )}
            <span className="min-w-0">
              <span className="block truncate font-medium">{thread.subject}</span>
              <span className="text-xs text-muted-foreground">
                {thread.messageCount} message{thread.messageCount !== 1 ? "s" : ""} ·{" "}
                {formatDateTime(thread.lastActivity)}
              </span>
            </span>
          </button>
          <div className="flex shrink-0 items-center gap-2">
            {anchor && (
              <TriageControl
                current={thread.currentLabel}
                onTriage={(label) => onTriage(anchor.messageId, thread.threadId, label)}
                testId={`triage-${thread.threadId}`}
              />
            )}
          </div>
        </div>
      </CardHeader>

      {expanded && (
        <CardContent className="space-y-3">
          {thread.messages.length === 0 ? (
            <p className="text-sm text-muted-foreground">No message bodies in this thread.</p>
          ) : (
            thread.messages.map((m) => <MessageRow key={m.id} message={m} />)
          )}

          <div className="flex justify-end">
            <Button
              variant="outline"
              size="sm"
              onClick={() => setReplyOpen((v) => !v)}
              data-testid={`inbox-reply-toggle-${thread.threadId}`}
            >
              <Reply className="mr-1.5 h-4 w-4" />
              {replyOpen ? "Hide reply" : "Draft reply"}
            </Button>
          </div>

          {replyOpen && (
            <DraftComposer
              threadId={thread.threadId}
              defaultTo={inbound?.from ? [inbound.from] : undefined}
              defaultSubject={
                thread.subject && thread.subject !== "(no subject)"
                  ? thread.subject.startsWith("Re:")
                    ? thread.subject
                    : `Re: ${thread.subject}`
                  : undefined
              }
              inReplyTo={inbound?.messageId}
              onDraft={onDraft}
              onSend={onSend}
            />
          )}
        </CardContent>
      )}
    </Card>
  );
}

interface MessageStreamProps {
  /** The selected connection's email, for the header. Empty → nothing selected. */
  connectionEmail?: string;
  threads: EmailThread[];
  isLoading: boolean;
  hasSelection: boolean;
  onTriage: (messageId: string, threadId: string, label: TriageLabel) => Promise<void>;
  onDraft: (req: DraftRequest) => Promise<DraftResponse>;
  onSend: (req: SendRequest) => Promise<void>;
}

export function MessageStream({
  connectionEmail,
  threads,
  isLoading,
  hasSelection,
  onTriage,
  onDraft,
  onSend,
}: MessageStreamProps) {
  if (!hasSelection) {
    return (
      <Card data-testid="inbox-stream-card">
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Mail className="h-5 w-5" />
            Email stream
          </CardTitle>
          <CardDescription>Select a connected mailbox above to view its threads.</CardDescription>
        </CardHeader>
        <CardContent>
          <div
            className="py-12 text-center text-sm text-muted-foreground"
            data-testid="inbox-stream-no-selection"
          >
            No mailbox selected.
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card data-testid="inbox-stream-card">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Mail className="h-5 w-5" />
          Email stream
        </CardTitle>
        <CardDescription>
          {connectionEmail
            ? `Threads for ${connectionEmail} — grouped from email.* events, newest first.`
            : "Threads grouped from email.* events, newest first."}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-3">
        {isLoading ? (
          <div className="space-y-3" data-testid="inbox-stream-skeleton">
            {Array.from({ length: 4 }).map((_, i) => (
              <Skeleton key={`stream-skeleton-${i}`} className="h-16 w-full" />
            ))}
          </div>
        ) : threads.length === 0 ? (
          <div
            className="py-12 text-center text-sm text-muted-foreground"
            data-testid="inbox-stream-empty"
          >
            No email events for this mailbox yet.
          </div>
        ) : (
          threads.map((thread) => (
            <ThreadCard
              key={thread.threadId}
              thread={thread}
              onTriage={onTriage}
              onDraft={onDraft}
              onSend={onSend}
            />
          ))
        )}
      </CardContent>
    </Card>
  );
}
