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
  Select,
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
  Textarea,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import {
  AlertTriangle,
  BellRing,
  CheckCircle2,
  Clock,
  Mail,
  MessageSquare,
  StickyNote,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  addNote,
  type CreateNoticeResult,
  createNotice,
  listNotes,
  type MessageTemplate,
  type NoteView,
  type NoticeSeverity,
  type SendMessageResult,
  sendMessage,
} from "@/lib/comms-api";

/**
 * Communicate panel (Pillar C, §9 Phase 6) — proactively help ONE tenant from
 * the 360: post an in-app notice, send an operator→tenant email, and add/read
 * internal support notes. Cohort/at-risk outreach lives on the dedicated
 * /outreach page (it reuses recovery-dialog.tsx for the blast-radius guard);
 * THIS panel is single-tenant only, so no dry-run/confirm gate is needed here.
 *
 * Every call is SAME-ORIGIN via the BFF (comms-api.ts). Opt-out / rate-limit
 * suppression (skip_reason) comes back as a SUCCESS ({skipped:true}); this panel
 * surfaces it explicitly (§ requirement 3) instead of failing silently.
 */

// Date guard (§6) — never call new Date() on a possibly-undefined API field.
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

const SEVERITIES: NoticeSeverity[] = ["info", "warning", "critical"];

const MESSAGE_TEMPLATES: { key: MessageTemplate; label: string }[] = [
  { key: "at_risk_outreach", label: "At-risk outreach" },
  { key: "quota_warning", label: "Quota warning (80/100%)" },
  { key: "onboarding_nudge", label: "Onboarding nudge" },
  { key: "dunning_reminder", label: "Dunning reminder" },
  { key: "custom", label: "Custom (subject + body)" },
];

interface CommsPanelProps {
  tenantId: string;
  tenantName: string;
}

export function CommsPanel({ tenantId, tenantName }: CommsPanelProps) {
  // ── In-app notice (single tenant) ───────────────────────────────────
  const [noticeTitle, setNoticeTitle] = useState("");
  const [noticeBody, setNoticeBody] = useState("");
  const [noticeSeverity, setNoticeSeverity] = useState<NoticeSeverity>("info");
  const [noticeExpiresAt, setNoticeExpiresAt] = useState("");
  const [noticeBusy, setNoticeBusy] = useState(false);
  const [noticeResult, setNoticeResult] = useState<CreateNoticeResult | null>(null);
  const [noticeError, setNoticeError] = useState<string | null>(null);

  const submitNotice = useCallback(async () => {
    setNoticeBusy(true);
    setNoticeError(null);
    setNoticeResult(null);
    try {
      const result = await createNotice({
        audience: { tenant_id: tenantId },
        title: noticeTitle.trim(),
        body: noticeBody.trim(),
        severity: noticeSeverity,
        expires_at: noticeExpiresAt.trim() || undefined,
      });
      setNoticeResult(result);
      setNoticeTitle("");
      setNoticeBody("");
      setNoticeExpiresAt("");
    } catch (err) {
      setNoticeError(err instanceof Error ? err.message : "Failed to post notice");
    } finally {
      setNoticeBusy(false);
    }
  }, [tenantId, noticeTitle, noticeBody, noticeSeverity, noticeExpiresAt]);

  // ── Operator → tenant email ─────────────────────────────────────────
  const [template, setTemplate] = useState<MessageTemplate>("at_risk_outreach");
  const [subject, setSubject] = useState("");
  const [messageBody, setMessageBody] = useState("");
  const [messageBusy, setMessageBusy] = useState(false);
  const [messageResult, setMessageResult] = useState<SendMessageResult | null>(null);
  const [messageError, setMessageError] = useState<string | null>(null);

  const isCustom = template === "custom";

  const submitMessage = useCallback(
    async (dryRun: boolean) => {
      setMessageBusy(true);
      setMessageError(null);
      setMessageResult(null);
      try {
        const result = await sendMessage({
          tenant_id: tenantId,
          // Always send the template key; for `custom` the CP requires subject+body.
          template,
          subject: isCustom ? subject.trim() : undefined,
          body: isCustom ? messageBody.trim() : undefined,
          dry_run: dryRun,
        });
        setMessageResult(result);
      } catch (err) {
        setMessageError(err instanceof Error ? err.message : "Failed to send message");
      } finally {
        setMessageBusy(false);
      }
    },
    [tenantId, template, isCustom, subject, messageBody]
  );

  // ── Support notes (internal) ────────────────────────────────────────
  const [notes, setNotes] = useState<NoteView[]>([]);
  const [noteBody, setNoteBody] = useState("");
  const [noteBusy, setNoteBusy] = useState(false);
  const [noteError, setNoteError] = useState<string | null>(null);

  const loadNotes = useCallback(async () => {
    try {
      setNotes(await listNotes(tenantId));
    } catch (err) {
      console.error("Failed to load support notes:", err);
      setNotes([]);
    }
  }, [tenantId]);

  useEffect(() => {
    loadNotes();
  }, [loadNotes]);

  const submitNote = useCallback(async () => {
    if (!noteBody.trim()) return;
    setNoteBusy(true);
    setNoteError(null);
    try {
      await addNote(tenantId, noteBody.trim());
      setNoteBody("");
      await loadNotes();
    } catch (err) {
      setNoteError(err instanceof Error ? err.message : "Failed to add note");
    } finally {
      setNoteBusy(false);
    }
  }, [tenantId, noteBody, loadNotes]);

  const noticeReady = noticeTitle.trim() !== "" && noticeBody.trim() !== "" && !noticeBusy;
  const messageReady =
    !messageBusy && (!isCustom || (subject.trim() !== "" && messageBody.trim() !== ""));

  return (
    <Card data-testid="tenant-comms-panel">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5" />
          Communicate
        </CardTitle>
        <CardDescription>
          Proactively help <span className="font-medium">{tenantName}</span>: post an in-app notice,
          send an operator email, or log an internal support note. Opt-out / rate-limit suppression
          is surfaced, not hidden. Cohort outreach lives on the{" "}
          <span className="font-mono">At-risk outreach</span> page.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-8">
        {/* ── Send in-app notice (single tenant) ─────────────────────── */}
        <section className="space-y-3" data-testid="comms-notice-section">
          <div className="flex items-center gap-2">
            <BellRing className="h-4 w-4 text-muted-foreground" />
            <h3 className="text-sm font-medium">Send in-app notice</h3>
          </div>
          <div className="grid gap-3 sm:grid-cols-2">
            <div className="space-y-1.5">
              <Label htmlFor="notice-title" className="text-xs text-muted-foreground">
                Title
              </Label>
              <Input
                id="notice-title"
                value={noticeTitle}
                onChange={(e) => setNoticeTitle(e.target.value)}
                placeholder="Scheduled maintenance window"
                data-testid="comms-notice-title"
              />
            </div>
            <div className="space-y-1.5">
              <Label htmlFor="notice-severity" className="text-xs text-muted-foreground">
                Severity
              </Label>
              <Select
                id="notice-severity"
                value={noticeSeverity}
                onChange={(e) => setNoticeSeverity(e.target.value as NoticeSeverity)}
                data-testid="comms-notice-severity"
              >
                {SEVERITIES.map((s) => (
                  <option key={s} value={s}>
                    {s}
                  </option>
                ))}
              </Select>
            </div>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="notice-body" className="text-xs text-muted-foreground">
              Body
            </Label>
            <Textarea
              id="notice-body"
              value={noticeBody}
              onChange={(e) => setNoticeBody(e.target.value)}
              placeholder="We'll be performing maintenance on…"
              data-testid="comms-notice-body"
            />
          </div>
          <div className="grid gap-3 sm:grid-cols-2 sm:items-end">
            <div className="space-y-1.5">
              <Label htmlFor="notice-expires" className="text-xs text-muted-foreground">
                Expires at (optional, ISO)
              </Label>
              <Input
                id="notice-expires"
                value={noticeExpiresAt}
                onChange={(e) => setNoticeExpiresAt(e.target.value)}
                placeholder="2026-12-31T23:59:59Z"
                autoComplete="off"
                data-testid="comms-notice-expires"
              />
            </div>
            <div className="flex justify-end">
              <Button
                onClick={submitNotice}
                disabled={!noticeReady}
                data-testid="comms-notice-send-btn"
              >
                <BellRing className="mr-2 h-4 w-4" />
                {noticeBusy ? "Posting…" : "Post notice"}
              </Button>
            </div>
          </div>

          {noticeError && (
            <p className="text-sm text-red-500" data-testid="comms-notice-error">
              {noticeError}
            </p>
          )}
          {noticeResult && (
            <div
              className="flex items-start gap-2 rounded-lg border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-600"
              data-testid="comms-notice-success"
            >
              <CheckCircle2 className="h-4 w-4 shrink-0 mt-0.5" />
              <span>
                Notice posted to <strong>{tenantName}</strong>
                {typeof noticeResult.count === "number" ? ` (${noticeResult.count})` : ""}. It will
                render in the tenant&apos;s dashboard banner.
              </span>
            </div>
          )}
        </section>

        {/* ── Operator → tenant email ────────────────────────────────── */}
        <section className="space-y-3" data-testid="comms-message-section">
          <div className="flex items-center gap-2">
            <Mail className="h-4 w-4 text-muted-foreground" />
            <h3 className="text-sm font-medium">Send operator email</h3>
          </div>
          <div className="space-y-1.5">
            <Label htmlFor="message-template" className="text-xs text-muted-foreground">
              Template
            </Label>
            <Select
              id="message-template"
              value={template}
              onChange={(e) => setTemplate(e.target.value as MessageTemplate)}
              data-testid="comms-message-template"
            >
              {MESSAGE_TEMPLATES.map((t) => (
                <option key={t.key} value={t.key}>
                  {t.label}
                </option>
              ))}
            </Select>
          </div>
          {isCustom && (
            <div className="space-y-3" data-testid="comms-message-custom">
              <div className="space-y-1.5">
                <Label htmlFor="message-subject" className="text-xs text-muted-foreground">
                  Subject
                </Label>
                <Input
                  id="message-subject"
                  value={subject}
                  onChange={(e) => setSubject(e.target.value)}
                  placeholder="A quick note about your workspace"
                  data-testid="comms-message-subject"
                />
              </div>
              <div className="space-y-1.5">
                <Label htmlFor="message-body" className="text-xs text-muted-foreground">
                  Body
                </Label>
                <Textarea
                  id="message-body"
                  value={messageBody}
                  onChange={(e) => setMessageBody(e.target.value)}
                  placeholder="Hi — we noticed…"
                  data-testid="comms-message-body"
                />
              </div>
            </div>
          )}
          <div className="flex flex-wrap items-center gap-2">
            <Button
              variant="outline"
              onClick={() => submitMessage(true)}
              disabled={!messageReady}
              data-testid="comms-message-preview-btn"
            >
              {messageBusy ? "Working…" : "Preview (dry-run)"}
            </Button>
            <Button
              onClick={() => submitMessage(false)}
              disabled={!messageReady}
              data-testid="comms-message-send-btn"
            >
              <Mail className="mr-2 h-4 w-4" />
              {messageBusy ? "Sending…" : "Send email"}
            </Button>
          </div>

          {messageError && (
            <p className="text-sm text-red-500" data-testid="comms-message-error">
              {messageError}
            </p>
          )}
          {messageResult && <MessageOutcome result={messageResult} />}
        </section>

        {/* ── Support notes (internal) ───────────────────────────────── */}
        <section className="space-y-3" data-testid="comms-notes-section">
          <div className="flex items-center gap-2">
            <StickyNote className="h-4 w-4 text-muted-foreground" />
            <h3 className="text-sm font-medium">Support notes (internal)</h3>
          </div>
          <div className="space-y-1.5">
            <Textarea
              value={noteBody}
              onChange={(e) => setNoteBody(e.target.value)}
              placeholder="Internal note — never sent to the tenant…"
              data-testid="comms-note-body"
            />
          </div>
          <div className="flex justify-end">
            <Button
              variant="outline"
              onClick={submitNote}
              disabled={noteBusy || noteBody.trim() === ""}
              data-testid="comms-note-add-btn"
            >
              <StickyNote className="mr-2 h-4 w-4" />
              {noteBusy ? "Adding…" : "Add note"}
            </Button>
          </div>
          {noteError && (
            <p className="text-sm text-red-500" data-testid="comms-note-error">
              {noteError}
            </p>
          )}

          {notes.length === 0 ? (
            <p className="text-sm text-muted-foreground" data-testid="comms-notes-empty">
              No support notes yet.
            </p>
          ) : (
            <Table data-testid="comms-notes-table">
              <TableHeader>
                <TableRow>
                  <TableHead>Note</TableHead>
                  <TableHead>Actor</TableHead>
                  <TableHead>When</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {notes.map((n) => (
                  <TableRow key={n.id} data-testid={`comms-note-row-${n.id}`}>
                    <TableCell className="max-w-md whitespace-pre-wrap">{n.body}</TableCell>
                    <TableCell className="text-muted-foreground">{n.actor || "—"}</TableCell>
                    <TableCell className="whitespace-nowrap">
                      {formatDateTime(n.created_at)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          )}
        </section>
      </CardContent>
    </Card>
  );
}

/**
 * Render the email send outcome — explicitly distinguishing SENT, a DRY-RUN
 * preview, and a SKIPPED send (opt-out / rate-limit). A skip is a success
 * response, not an error, so it gets its own visible, non-red treatment with the
 * skip_reason spelled out (§ requirement 3).
 */
function MessageOutcome({ result }: { result: SendMessageResult }) {
  if (result.skipped) {
    const reason = result.skip_reason ?? "skipped";
    const optOut = reason === "skipped_opt_out";
    return (
      <div
        className="flex items-start gap-2 rounded-lg border border-yellow-500/30 bg-yellow-500/10 p-3 text-sm text-yellow-600"
        data-testid="comms-message-skipped"
      >
        <Clock className="h-4 w-4 shrink-0 mt-0.5" />
        <div className="space-y-1">
          <p className="font-medium">
            Not sent — {optOut ? "tenant opted out" : "rate-limited"}{" "}
            <Badge variant="secondary" className="ml-1 font-mono text-[10px]">
              {reason}
            </Badge>
          </p>
          <p className="text-xs">
            {optOut
              ? "This tenant has opted out of operator email for this category. The skip is recorded in the audit ledger."
              : "This template was sent too recently (per-template cooldown). Try again after the cooldown window."}
          </p>
        </div>
      </div>
    );
  }

  if (result.dry_run) {
    return (
      <div
        className="flex items-start gap-2 rounded-lg border p-3 text-sm"
        data-testid="comms-message-dryrun"
      >
        <AlertTriangle className="h-4 w-4 shrink-0 mt-0.5 text-muted-foreground" />
        <div className="space-y-1">
          <p className="font-medium">Dry-run — nothing sent.</p>
          <p className="text-xs text-muted-foreground">
            Would email <span className="font-mono">{result.recipient || "—"}</span>
            {result.subject ? (
              <>
                {" "}
                — subject &ldquo;<span className="italic">{result.subject}</span>&rdquo;
              </>
            ) : null}
            . Send for real to deliver via SMTP (emits{" "}
            <span className="font-mono">admin.message.sent</span>).
          </p>
        </div>
      </div>
    );
  }

  return (
    <div
      className={cn(
        "flex items-start gap-2 rounded-lg border p-3 text-sm",
        result.sent
          ? "border-green-500/30 bg-green-500/10 text-green-600"
          : "border-muted text-muted-foreground"
      )}
      data-testid="comms-message-sent"
    >
      <CheckCircle2 className="h-4 w-4 shrink-0 mt-0.5" />
      <span>
        {result.sent ? "Email sent" : "Processed"} to{" "}
        <span className="font-mono">{result.recipient || "—"}</span>
        {result.subject ? (
          <>
            {" "}
            — &ldquo;<span className="italic">{result.subject}</span>&rdquo;
          </>
        ) : null}
        . Audited as <span className="font-mono">admin.message.sent</span>.
      </span>
    </div>
  );
}
