"use client";

import { Button, Input, Label, Select, Textarea } from "@allsource/ui";
import { Turnstile, type TurnstileInstance } from "@marsidev/react-turnstile";
import { AlertCircle, ArrowRight, CheckCircle2, Loader2 } from "lucide-react";
import Link from "next/link";
import { useEffect, useRef, useState } from "react";

const TURNSTILE_SITE_KEY = process.env.NEXT_PUBLIC_TURNSTILE_SITE_KEY || "";
const fieldClassName =
  "min-h-11 border-slate-400 bg-white text-[#122033] placeholder:text-slate-500 focus-visible:border-[#0C69C7] focus-visible:ring-2 focus-visible:ring-[#0C69C7] focus-visible:ring-offset-2 focus-visible:ring-offset-[#F7F9FC] dark:bg-white";

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
  const errorAlert = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (error) {
      errorAlert.current?.focus();
    }
  }, [error]);

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
        className="rounded-2xl border border-[#33C6D0]/45 bg-[#F7F9FC] p-7 text-[#122033] shadow-xl shadow-black/20 sm:p-9"
        role="status"
      >
        <div className="flex h-12 w-12 items-center justify-center rounded-full bg-[#33C6D0]/15">
          <CheckCircle2 className="h-6 w-6 text-[#087C72]" aria-hidden="true" />
        </div>
        <h2 className="mt-7 text-3xl font-semibold tracking-tight">Application received.</h2>
        <p className="mt-4 max-w-xl leading-7 text-slate-600">
          We&apos;ll read every application and reply by email within five business days. No review,
          endorsement, or public post is expected.
        </p>
        <p className="mt-7 font-mono text-xs text-slate-600">reference · {applicationID}</p>
        <Link
          href="/docs/prime/quickstart"
          className="mt-8 inline-flex min-h-6 items-center gap-2 font-medium text-[#0C69C7] underline decoration-[#0C69C7]/35 underline-offset-4 focus-visible:rounded-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#0C69C7] focus-visible:ring-offset-2"
        >
          Explore Prime while you wait <ArrowRight className="h-4 w-4" />
        </Link>
      </div>
    );
  }

  return (
    <form
      onSubmit={handleSubmit}
      className="rounded-2xl border border-slate-300 bg-[#F7F9FC] p-5 text-[#122033] shadow-xl shadow-black/20 sm:p-8"
      aria-busy={pending}
      aria-labelledby="design-partner-form-title"
    >
      <div className="border-b border-slate-300 pb-5">
        <p className="text-sm font-semibold text-[#0C69C7]">Apply for one of five places</p>
        <h2
          id="design-partner-form-title"
          className="mt-2 text-2xl font-semibold tracking-[-0.02em] sm:text-3xl"
        >
          Show us one failing memory flow.
        </h2>
        <p className="mt-2 text-sm leading-6 text-slate-600">
          Usually takes three minutes. Every field is required.
        </p>
      </div>

      {error && (
        <div
          ref={errorAlert}
          className="mt-5 flex items-start gap-3 rounded-lg border border-red-300 bg-red-50 p-4 text-sm text-red-900 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-red-700 focus-visible:ring-offset-2"
          role="alert"
          tabIndex={-1}
        >
          <AlertCircle className="mt-0.5 h-4 w-4 shrink-0" aria-hidden="true" />
          <span>{error}</span>
        </div>
      )}

      <div className="mt-5 grid gap-4 sm:grid-cols-2">
        <Field label="Your name" htmlFor="dp-name">
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

      <div className="mt-4">
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
            placeholder="Project, product, or agent name"
          />
        </Field>
      </div>

      <div className="mt-4">
        <Field
          label="What does the agent do?"
          htmlFor="dp-use-case"
          hint="30–1,000 characters · Who uses it, what job it runs, and what must persist."
        >
          <Textarea
            id="dp-use-case"
            name="agent_use_case"
            className={fieldClassName}
            aria-describedby="dp-use-case-hint"
            minLength={30}
            maxLength={1000}
            rows={4}
            required
            disabled={pending}
            placeholder="Describe its users, job, and cross-session state."
          />
        </Field>
      </div>

      <div className="mt-4">
        <Field
          label="Where does memory fail?"
          htmlFor="dp-memory-problem"
          hint="30–1,000 characters · Name one failure: lost context, stale recall, missing source, or no historical state."
        >
          <Textarea
            id="dp-memory-problem"
            name="memory_problem"
            className={fieldClassName}
            aria-describedby="dp-memory-problem-hint"
            minLength={30}
            maxLength={1000}
            rows={4}
            required
            disabled={pending}
            placeholder="Describe what fails today and how you notice."
          />
        </Field>
      </div>

      <div className="mt-4">
        <Field
          label="When can you integrate?"
          htmlFor="dp-timeline"
          hint="Choose the earliest realistic start."
        >
          <Select
            id="dp-timeline"
            name="timeline"
            aria-describedby="dp-timeline-hint"
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

      <label className="mt-5 flex min-h-11 cursor-pointer items-start gap-3 rounded-lg border border-slate-300 bg-white p-4 text-sm leading-6 text-slate-700">
        <input
          name="consent"
          value="yes"
          type="checkbox"
          aria-label="I consent to application review and contact."
          aria-describedby="dp-consent-copy"
          required
          disabled={pending}
          className="h-6 w-6 shrink-0 rounded border-slate-400 accent-[#0C69C7] focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#0C69C7] focus-visible:ring-offset-2"
        />
        <span id="dp-consent-copy">
          AllSource may use these answers to review my application and contact me about this
          program. See{" "}
          <Link
            href="/privacy#design-partner-applications"
            className="rounded-sm font-medium text-[#0C69C7] underline underline-offset-2 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-[#0C69C7] focus-visible:ring-offset-2"
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
        className="mt-5 min-h-12 w-full bg-[#0C69C7] text-white hover:bg-[#0959A9] focus-visible:ring-2 focus-visible:ring-[#0C69C7] focus-visible:ring-offset-2 focus-visible:ring-offset-[#F7F9FC]"
      >
        {pending ? (
          <>
            <Loader2 className="mr-2 h-4 w-4 animate-spin" aria-hidden="true" />
            Submitting…
          </>
        ) : (
          <>
            Send design-partner application
            <ArrowRight className="ml-2 h-4 w-4" aria-hidden="true" />
          </>
        )}
      </Button>
      <p className="mt-4 text-xs leading-5 text-slate-600">
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
      <Label htmlFor={htmlFor} className="font-medium text-slate-900">
        {label}
      </Label>
      {children}
      {hint && (
        <p id={`${htmlFor}-hint`} className="text-xs leading-5 text-slate-600">
          {hint}
        </p>
      )}
    </div>
  );
}
