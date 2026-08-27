"use client";

import { Button, Input, Label, Select, Textarea } from "@allsource/ui";
import { Turnstile, type TurnstileInstance } from "@marsidev/react-turnstile";
import { AlertCircle, ArrowRight, CheckCircle2, Loader2 } from "lucide-react";
import Link from "next/link";
import { useRef, useState } from "react";

const TURNSTILE_SITE_KEY = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY || "";
const fieldClassName =
  "border-slate-300 bg-white text-[#0E1A2A] placeholder:text-slate-400 focus-visible:border-[#2F8CFF] focus-visible:ring-[#2F8CFF]/30 dark:bg-white";

interface CampaignSource {
  source: string;
  medium: string;
  campaign: string;
  content: string;
  term: string;
}

interface DesignPartnerFormProps {
  campaignSource: CampaignSource;
}

interface ApplicationResponse {
  application_id?: string;
  message?: string;
  retry_after?: number;
}

export function DesignPartnerForm({ campaignSource }: DesignPartnerFormProps) {
  const [pending, setPending] = useState(false);
  const [error, setError] = useState("");
  const [applicationID, setApplicationID] = useState("");
  const [turnstileToken, setTurnstileToken] = useState("");
  const idempotencyKey = useRef("");
  const turnstile = useRef<TurnstileInstance>(null);

  async function handleSubmit(event: React.FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError("");
    setPending(true);
    const form = new FormData(event.currentTarget);
    if (!idempotencyKey.current) {
      idempotencyKey.current = crypto.randomUUID();
    }

    try {
      const response = await fetch("/api/design-partners/applications", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          name: form.get("name"),
          email: form.get("email"),
          project: form.get("project"),
          agent_use_case: form.get("agent_use_case"),
          memory_problem: form.get("memory_problem"),
          timeline: form.get("timeline"),
          consent: form.get("consent") === "yes",
          idempotency_key: idempotencyKey.current,
          campaign_source: campaignSource,
          ...(turnstileToken && { cf_turnstile_response: turnstileToken }),
        }),
      });
      const data = (await response.json().catch(() => ({}))) as ApplicationResponse;
      if (!response.ok) {
        if (response.status === 429) {
          const minutes = Math.max(1, Math.ceil((data.retry_after || 60) / 60));
          throw new Error(
            `Too many attempts. Try again in about ${minutes} minute${minutes === 1 ? "" : "s"}.`
          );
        }
        throw new Error(data.message || "Application could not be submitted. Please try again.");
      }
      setApplicationID(data.application_id || "received");
    } catch (submissionError) {
      setError(
        submissionError instanceof Error
          ? submissionError.message
          : "Application could not be submitted. Please try again."
      );
      turnstile.current?.reset();
      setTurnstileToken("");
    } finally {
      setPending(false);
    }
  }

  if (applicationID) {
    return (
      <div
        className="rounded-[1.5rem] border border-[#38D6C8]/35 bg-[#F4F7FB] p-8 text-[#0E1A2A] shadow-2xl shadow-black/25 sm:p-10"
        role="status"
      >
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-[#38D6C8]/15">
          <CheckCircle2 className="h-6 w-6 text-[#087C72]" aria-hidden="true" />
        </div>
        <p className="mt-7 font-mono text-xs uppercase tracking-[0.18em] text-[#087C72]">
          application_submitted
        </p>
        <h2 className="mt-3 text-3xl font-semibold tracking-tight">
          You&apos;re in the review queue.
        </h2>
        <p className="mt-4 max-w-xl leading-7 text-slate-600">
          We&apos;ll read every application and reply by email within five business days. No review,
          endorsement, or public post is expected.
        </p>
        <p className="mt-7 font-mono text-xs text-slate-500">reference · {applicationID}</p>
        <Link
          href="/docs/prime/quickstart"
          className="mt-8 inline-flex items-center gap-2 font-medium text-[#176FD1] underline decoration-[#176FD1]/30 underline-offset-4 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#2F8CFF]"
        >
          Explore Prime while you wait <ArrowRight className="h-4 w-4" />
        </Link>
      </div>
    );
  }

  return (
    <form
      onSubmit={handleSubmit}
      className="rounded-[1.5rem] border border-white/15 bg-[#F4F7FB] p-6 text-[#0E1A2A] shadow-2xl shadow-black/30 sm:p-9"
      aria-busy={pending}
    >
      <div className="flex items-start justify-between gap-4 border-b border-slate-200 pb-6">
        <div>
          <p className="font-mono text-[11px] uppercase tracking-[0.18em] text-[#176FD1]">Apply</p>
          <h2 className="mt-2 text-2xl font-semibold tracking-tight sm:text-3xl">
            Tell us what memory must do.
          </h2>
        </div>
        <span className="shrink-0 rounded-full border border-[#F0B44D]/40 bg-[#F0B44D]/10 px-3 py-1 font-mono text-[10px] uppercase tracking-wider text-[#8A5A00]">
          3 min
        </span>
      </div>

      {error && (
        <div
          className="mt-6 flex items-start gap-3 rounded-xl border border-red-200 bg-red-50 p-4 text-sm text-red-800"
          role="alert"
        >
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}

      <div className="mt-7 grid gap-5 sm:grid-cols-2">
        <Field label="Name" htmlFor="dp-name">
          <Input
            id="dp-name"
            name="name"
            className={fieldClassName}
            autoComplete="name"
            minLength={2}
            maxLength={80}
            required
            disabled={pending}
          />
        </Field>
        <Field label="Work email" htmlFor="dp-email">
          <Input
            id="dp-email"
            name="email"
            className={fieldClassName}
            type="email"
            autoComplete="email"
            maxLength={254}
            required
            disabled={pending}
          />
        </Field>
      </div>

      <div className="mt-5">
        <Field label="Project or company" htmlFor="dp-project">
          <Input
            id="dp-project"
            name="project"
            className={fieldClassName}
            autoComplete="organization"
            minLength={2}
            maxLength={120}
            required
            disabled={pending}
            placeholder="What are you building?"
          />
        </Field>
      </div>

      <div className="mt-5">
        <Field label="Agent use case" htmlFor="dp-use-case" hint="30–1,000 characters">
          <Textarea
            id="dp-use-case"
            name="agent_use_case"
            className={fieldClassName}
            minLength={30}
            maxLength={1000}
            rows={5}
            required
            disabled={pending}
            placeholder="What does the agent do, for whom, and what state must carry across sessions?"
          />
        </Field>
      </div>

      <div className="mt-5">
        <Field
          label="Current memory problem"
          htmlFor="dp-memory-problem"
          hint="30–1,000 characters"
        >
          <Textarea
            id="dp-memory-problem"
            name="memory_problem"
            className={fieldClassName}
            minLength={30}
            maxLength={1000}
            rows={5}
            required
            disabled={pending}
            placeholder="Where do summaries, vectors, local files, or existing memory tools break down?"
          />
        </Field>
      </div>

      <div className="mt-5">
        <Field label="Integration timeline" htmlFor="dp-timeline">
          <Select
            id="dp-timeline"
            name="timeline"
            required
            disabled={pending}
            defaultValue=""
            className={fieldClassName}
          >
            <option value="" disabled>
              Choose one
            </option>
            <option value="ready_now">Ready now</option>
            <option value="within_30_days">Within 30 days</option>
            <option value="within_60_days">Within 60 days</option>
            <option value="exploring">Exploring; timing not fixed</option>
          </Select>
        </Field>
      </div>

      <label className="mt-6 flex cursor-pointer items-start gap-3 rounded-xl border border-slate-200 bg-white p-4 text-sm leading-6 text-slate-600">
        <input
          name="consent"
          value="yes"
          type="checkbox"
          required
          disabled={pending}
          className="mt-1 h-4 w-4 rounded border-slate-300 accent-[#176FD1] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#2F8CFF]"
        />
        <span>
          AllSource may use these answers to review my application and contact me about this
          program. See{" "}
          <Link
            href="/privacy#design-partner-applications"
            className="font-medium text-[#176FD1] underline underline-offset-2"
          >
            application privacy
          </Link>
          .
        </span>
      </label>

      {TURNSTILE_SITE_KEY && (
        <Turnstile
          ref={turnstile}
          siteKey={TURNSTILE_SITE_KEY}
          onSuccess={setTurnstileToken}
          onExpire={() => setTurnstileToken("")}
          onError={() => setError("Spam check could not load. Refresh and try again.")}
          options={{ size: "invisible", theme: "light" }}
        />
      )}

      <Button
        type="submit"
        disabled={pending || Boolean(TURNSTILE_SITE_KEY && !turnstileToken)}
        className="mt-6 h-12 w-full bg-[#176FD1] text-white hover:bg-[#115FAF] focus-visible:ring-[#2F8CFF]"
      >
        {pending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
            Submitting…
          </>
        ) : (
          <>
            Apply for one of five spots <ArrowRight className="ml-2 h-4 w-4" aria-hidden="true" />
          </>
        )}
      </Button>
      <p className="mt-4 text-center text-xs leading-5 text-slate-500">
        Applicant details stay in a private admin stream. We do not place them in public analytics,
        issues, or campaign URLs.
      </p>
    </form>
  );
}

function Field({
  label,
  htmlFor,
  hint,
  children,
}: {
  label: string;
  htmlFor: string;
  hint?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-2">
      <div className="flex items-end justify-between gap-3">
        <Label htmlFor={htmlFor} className="font-medium text-slate-800">
          {label}
        </Label>
        {hint && (
          <span className="font-mono text-[10px] uppercase tracking-wider text-slate-400">
            {hint}
          </span>
        )}
      </div>
      {children}
    </div>
  );
}
