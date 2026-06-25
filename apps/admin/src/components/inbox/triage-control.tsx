"use client";

import { Badge, Select } from "@allsource/ui";
import { useState } from "react";
import { type TriageLabel, triageLabelText } from "@/lib/inbox-api";

/**
 * Per-thread triage control: a label select that writes an email.triaged event.
 * The four labels mirror the CP's enum (needs-reply|fyi|spam|archive). The parent
 * owns the POST so it can refresh the stream + toast on the page level.
 */
const LABELS: TriageLabel[] = ["needs-reply", "fyi", "spam", "archive"];

function labelVariant(label?: TriageLabel): "default" | "secondary" | "destructive" | "outline" {
  switch (label) {
    case "needs-reply":
      return "default";
    case "fyi":
      return "secondary";
    case "spam":
      return "destructive";
    case "archive":
      return "outline";
    default:
      return "outline";
  }
}

interface TriageControlProps {
  current?: TriageLabel;
  /** Persist a triage label; resolves once the event is written. */
  onTriage: (label: TriageLabel) => Promise<void>;
  testId?: string;
}

export function TriageControl({ current, onTriage, testId }: TriageControlProps) {
  const [isSaving, setIsSaving] = useState(false);
  const [value, setValue] = useState<TriageLabel | "">(current ?? "");

  const handleChange = async (next: string) => {
    if (next === "" || next === value) return;
    const label = next as TriageLabel;
    setIsSaving(true);
    try {
      await onTriage(label);
      setValue(label);
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <div className="flex items-center gap-2" data-testid={testId ?? "triage-control"}>
      {current && (
        <Badge variant={labelVariant(current)} data-testid="triage-current-label">
          {triageLabelText(current)}
        </Badge>
      )}
      <Select
        value={value}
        onChange={(e) => handleChange(e.target.value)}
        disabled={isSaving}
        className="h-8 w-36"
        aria-label="Set triage label"
        data-testid="triage-select"
      >
        <option value="">{isSaving ? "Saving…" : "Triage…"}</option>
        {LABELS.map((l) => (
          <option key={l} value={l}>
            {triageLabelText(l)}
          </option>
        ))}
      </Select>
    </div>
  );
}
