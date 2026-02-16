"use client";

import { Button, Card, CardContent, Label, Textarea } from "@allsource/ui";
import {
  AlertCircle,
  CheckCircle,
  HelpCircle,
  Lightbulb,
  Loader2,
  MessageSquarePlus,
  X,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

type Category = "bug" | "feature" | "question";

const categories: { value: Category; label: string; icon: typeof AlertCircle }[] = [
  { value: "bug", label: "Bug Report", icon: AlertCircle },
  { value: "feature", label: "Feature Request", icon: Lightbulb },
  { value: "question", label: "Question", icon: HelpCircle },
];

const RATE_LIMIT_KEY = "feedback_timestamps";
const RATE_LIMIT_MAX = 5;
const RATE_LIMIT_WINDOW_MS = 60 * 60 * 1000; // 1 hour

function checkRateLimit(): boolean {
  try {
    const stored = localStorage.getItem(RATE_LIMIT_KEY);
    const timestamps: number[] = stored ? JSON.parse(stored) : [];
    const now = Date.now();
    const recent = timestamps.filter((t) => now - t < RATE_LIMIT_WINDOW_MS);
    return recent.length < RATE_LIMIT_MAX;
  } catch {
    return true;
  }
}

function recordSubmission(): void {
  try {
    const stored = localStorage.getItem(RATE_LIMIT_KEY);
    const timestamps: number[] = stored ? JSON.parse(stored) : [];
    const now = Date.now();
    const recent = timestamps.filter((t) => now - t < RATE_LIMIT_WINDOW_MS);
    recent.push(now);
    localStorage.setItem(RATE_LIMIT_KEY, JSON.stringify(recent));
  } catch {
    // localStorage unavailable
  }
}

export function FeedbackWidget() {
  const [open, setOpen] = useState(false);
  const [category, setCategory] = useState<Category>("bug");
  const [message, setMessage] = useState("");
  const [email, setEmail] = useState("");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const handleClose = useCallback(() => {
    setOpen(false);
    setError(null);
    if (!success) return;
    // Reset form after close if submitted successfully
    setCategory("bug");
    setMessage("");
    setEmail("");
    setSuccess(false);
  }, [success]);

  // Focus textarea when modal opens
  useEffect(() => {
    if (open && textareaRef.current) {
      textareaRef.current.focus();
    }
  }, [open]);

  // Close on Escape
  useEffect(() => {
    if (!open) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") handleClose();
    };
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [open, handleClose]);

  const handleSubmit = async () => {
    if (!message.trim()) return;

    if (!checkRateLimit()) {
      setError("Rate limit reached. Please try again later (max 5 per hour).");
      return;
    }

    setIsSubmitting(true);
    setError(null);

    try {
      const res = await fetch("/api/feedback", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          category,
          message: message.trim(),
          email: email.trim() || undefined,
        }),
      });

      if (!res.ok) {
        const data = await res.json().catch(() => null);
        throw new Error(data?.error || "Failed to submit feedback");
      }

      recordSubmission();
      setSuccess(true);
      setTimeout(() => {
        handleClose();
      }, 2000);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to submit feedback");
    } finally {
      setIsSubmitting(false);
    }
  };

  return (
    <>
      {/* Floating feedback button */}
      <Button
        onClick={() => setOpen(true)}
        className="fixed bottom-6 right-6 z-30 h-12 w-12 rounded-full shadow-lg"
        size="icon"
        aria-label="Send feedback"
      >
        <MessageSquarePlus className="h-5 w-5" />
      </Button>

      {/* Modal */}
      {open && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <button
            type="button"
            className="absolute inset-0 bg-background/80 backdrop-blur-sm"
            onClick={handleClose}
            aria-label="Close dialog"
          />
          <Card className="relative z-10 w-full max-w-md mx-4">
            <CardContent className="p-6">
              <div className="mb-6 flex items-center justify-between">
                <div>
                  <h2 className="text-lg font-semibold">Send Feedback</h2>
                  <p className="text-sm text-muted-foreground">
                    Report a bug, request a feature, or ask a question
                  </p>
                </div>
                <Button variant="ghost" size="icon" onClick={handleClose} className="h-8 w-8">
                  <X className="h-4 w-4" />
                </Button>
              </div>

              {success ? (
                <div className="flex flex-col items-center py-8">
                  <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-500/10">
                    <CheckCircle className="h-8 w-8 text-green-500" />
                  </div>
                  <h3 className="mb-1 text-lg font-medium">Thank you!</h3>
                  <p className="text-center text-sm text-muted-foreground">
                    Your feedback has been submitted successfully.
                  </p>
                </div>
              ) : (
                <div className="space-y-4">
                  {/* Category selector */}
                  <div className="space-y-2">
                    <Label>Category</Label>
                    <div className="flex gap-2">
                      {categories.map((c) => (
                        <button
                          key={c.value}
                          type="button"
                          className={`flex flex-1 items-center justify-center gap-1.5 rounded-lg border p-2 text-sm transition-colors ${
                            category === c.value
                              ? "border-primary bg-primary/5 text-primary"
                              : "border-border text-muted-foreground hover:border-muted-foreground/30"
                          }`}
                          onClick={() => setCategory(c.value)}
                          disabled={isSubmitting}
                        >
                          <c.icon className="h-4 w-4" />
                          {c.label}
                        </button>
                      ))}
                    </div>
                  </div>

                  {/* Message */}
                  <div className="space-y-2">
                    <Label htmlFor="feedback-message">Message</Label>
                    <Textarea
                      ref={textareaRef}
                      id="feedback-message"
                      placeholder={
                        category === "bug"
                          ? "Describe the bug and steps to reproduce..."
                          : category === "feature"
                            ? "Describe the feature you'd like to see..."
                            : "What would you like to know?"
                      }
                      value={message}
                      onChange={(e) => setMessage(e.target.value)}
                      disabled={isSubmitting}
                      rows={4}
                      className="resize-none"
                    />
                  </div>

                  {/* Optional email */}
                  <div className="space-y-2">
                    <Label htmlFor="feedback-email">
                      Email <span className="text-muted-foreground">(optional)</span>
                    </Label>
                    <input
                      id="feedback-email"
                      type="email"
                      placeholder="you@example.com"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      disabled={isSubmitting}
                      className="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm ring-offset-background placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-50"
                    />
                  </div>

                  {error && (
                    <div className="rounded-lg border border-destructive/20 bg-destructive/10 p-3">
                      <p className="text-sm text-destructive">{error}</p>
                    </div>
                  )}

                  <div className="flex gap-2 pt-2">
                    <Button variant="outline" className="flex-1" onClick={handleClose}>
                      Cancel
                    </Button>
                    <Button
                      className="flex-1"
                      onClick={handleSubmit}
                      disabled={!message.trim() || isSubmitting}
                    >
                      {isSubmitting ? (
                        <>
                          <Loader2 className="mr-1.5 h-4 w-4 animate-spin" />
                          Submitting...
                        </>
                      ) : (
                        "Submit Feedback"
                      )}
                    </Button>
                  </div>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      )}
    </>
  );
}
