"use client";

import {
  Badge,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Input,
  Label,
  Textarea,
} from "@allsource/ui";
import { FileText, Send } from "lucide-react";
import { useState } from "react";
import { SendDialog } from "@/components/inbox/send-dialog";
import type { DraftRequest, DraftResponse, EmailAddress, SendRequest } from "@/lib/inbox-api";

/**
 * Draft + send composer for a thread. Two-step by design:
 *   1. Draft  → POST /draft (writes email.drafted, returns draft_id). Non-
 *      destructive; the composer shows the returned draft_id as a "drafted" state.
 *   2. Send   → opens the confirm dialog, then POST /send with confirm:true
 *      (never one-click). The send is linked to the draft_id when one exists.
 *
 * The parent owns the actual API calls (so it can refresh the stream + toast);
 * this component just collects fields, parses the To list, and orchestrates the
 * draft → confirm → send sequence.
 */
interface DraftComposerProps {
  threadId: string;
  /** Pre-fill recipients (e.g. the inbound sender) and subject (Re: …). */
  defaultTo?: EmailAddress[];
  defaultSubject?: string;
  /** message_id to thread the reply onto (in_reply_to). */
  inReplyTo?: string;
  onDraft: (req: DraftRequest) => Promise<DraftResponse>;
  onSend: (req: SendRequest) => Promise<void>;
}

/** Parse a comma/newline-separated recipient string into addresses. */
function parseRecipients(raw: string): EmailAddress[] {
  return raw
    .split(/[,\n]/)
    .map((s) => s.trim())
    .filter((s) => s !== "")
    .map((email) => ({ email }));
}

function formatRecipients(addrs?: EmailAddress[]): string {
  if (!addrs || addrs.length === 0) return "";
  return addrs.map((a) => a.email).join(", ");
}

export function DraftComposer({
  threadId,
  defaultTo,
  defaultSubject,
  inReplyTo,
  onDraft,
  onSend,
}: DraftComposerProps) {
  const [intent, setIntent] = useState("reply");
  const [subject, setSubject] = useState(defaultSubject ?? "");
  const [toRaw, setToRaw] = useState(formatRecipients(defaultTo));
  const [body, setBody] = useState("");
  const [isDrafting, setIsDrafting] = useState(false);
  const [draftId, setDraftId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sendOpen, setSendOpen] = useState(false);

  const to = parseRecipients(toRaw);

  const handleDraft = async () => {
    if (intent.trim() === "" || body.trim() === "") {
      setError("Intent and body are required to draft.");
      return;
    }
    setIsDrafting(true);
    setError(null);
    try {
      const res = await onDraft({
        tenant_id: "", // filled by the parent (it owns the tenant context)
        thread_id: threadId,
        body: body.trim(),
        intent: intent.trim(),
        in_reply_to: inReplyTo,
        subject: subject.trim() || undefined,
        to: to.length > 0 ? to : undefined,
      });
      setDraftId(res.draft_id);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to save draft.");
    } finally {
      setIsDrafting(false);
    }
  };

  return (
    <Card data-testid="inbox-draft-composer">
      <CardHeader>
        <CardTitle className="flex items-center gap-2 text-base">
          <FileText className="h-4 w-4" />
          Reply
          {draftId && (
            <Badge variant="secondary" data-testid="draft-saved-badge">
              Drafted · {draftId}
            </Badge>
          )}
        </CardTitle>
        <CardDescription>
          Draft a reply (writes an email.drafted event), then send it behind a confirm.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor={`draft-intent-${threadId}`}>Intent</Label>
            <Input
              id={`draft-intent-${threadId}`}
              value={intent}
              onChange={(e) => setIntent(e.target.value)}
              placeholder="reply / follow-up / decline"
              data-testid="draft-intent-input"
            />
          </div>
          <div className="space-y-2">
            <Label htmlFor={`draft-subject-${threadId}`}>Subject (optional)</Label>
            <Input
              id={`draft-subject-${threadId}`}
              value={subject}
              onChange={(e) => setSubject(e.target.value)}
              placeholder="Re: …"
              data-testid="draft-subject-input"
            />
          </div>
        </div>

        <div className="space-y-2">
          <Label htmlFor={`draft-to-${threadId}`}>To (optional, comma-separated)</Label>
          <Input
            id={`draft-to-${threadId}`}
            value={toRaw}
            onChange={(e) => setToRaw(e.target.value)}
            placeholder="someone@example.com, other@example.com"
            data-testid="draft-to-input"
          />
        </div>

        <div className="space-y-2">
          <Label htmlFor={`draft-body-${threadId}`}>Body</Label>
          <Textarea
            id={`draft-body-${threadId}`}
            value={body}
            onChange={(e) => setBody(e.target.value)}
            rows={6}
            placeholder="Write the reply…"
            data-testid="draft-body-input"
          />
        </div>

        {error && (
          <p className="text-sm text-red-500" data-testid="draft-error">
            {error}
          </p>
        )}

        <div className="flex items-center justify-end gap-2">
          <Button
            variant="outline"
            onClick={handleDraft}
            disabled={isDrafting}
            data-testid="draft-save-btn"
          >
            <FileText className="mr-1.5 h-4 w-4" />
            {isDrafting ? "Drafting…" : draftId ? "Re-draft" : "Save draft"}
          </Button>
          <Button
            onClick={() => setSendOpen(true)}
            disabled={to.length === 0 || body.trim() === ""}
            data-testid="draft-open-send-btn"
            title={to.length === 0 ? "Add at least one recipient to send" : undefined}
          >
            <Send className="mr-1.5 h-4 w-4" />
            Send…
          </Button>
        </div>
      </CardContent>

      <SendDialog
        open={sendOpen}
        onClose={() => setSendOpen(false)}
        to={to}
        subject={subject}
        body={body}
        draftId={draftId ?? undefined}
        onConfirm={async () => {
          await onSend({
            tenant_id: "", // filled by the parent
            to,
            body: body.trim(),
            subject: subject.trim() || undefined,
            thread_id: threadId,
            in_reply_to: inReplyTo,
            draft_id: draftId ?? undefined,
            confirm: true,
          });
        }}
      />
    </Card>
  );
}
