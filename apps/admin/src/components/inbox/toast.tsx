"use client";

import { cn } from "@allsource/ui/utils";
import { CheckCircle2, X, XCircle } from "lucide-react";
import { useCallback, useEffect, useState } from "react";

/**
 * A tiny self-contained toast for the Inbox page. The admin app ships no global
 * toast system (existing pages use inline error states); rather than introduce a
 * new shared dependency this keeps the toast scoped to components/inbox/* — the
 * only place the prompt asks for one (connect round-trip + action confirmations).
 */
export type ToastVariant = "success" | "error";

export interface ToastMessage {
  id: number;
  variant: ToastVariant;
  text: string;
}

let nextId = 1;

/** Hook that owns a toast queue + auto-dismiss. Returns push helpers + the stack. */
export function useToasts(autoDismissMs = 5000) {
  const [toasts, setToasts] = useState<ToastMessage[]>([]);

  const dismiss = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  const push = useCallback(
    (variant: ToastVariant, text: string) => {
      const id = nextId++;
      setToasts((prev) => [...prev, { id, variant, text }]);
      if (autoDismissMs > 0) {
        setTimeout(() => dismiss(id), autoDismissMs);
      }
    },
    [autoDismissMs, dismiss]
  );

  const toastSuccess = useCallback((text: string) => push("success", text), [push]);
  const toastError = useCallback((text: string) => push("error", text), [push]);

  return { toasts, toastSuccess, toastError, dismiss };
}

interface ToasterProps {
  toasts: ToastMessage[];
  onDismiss: (id: number) => void;
}

/** Renders the toast stack in a fixed bottom-right viewport. */
export function Toaster({ toasts, onDismiss }: ToasterProps) {
  return (
    <div
      className="fixed bottom-4 right-4 z-[60] flex w-full max-w-sm flex-col gap-2"
      data-testid="inbox-toaster"
    >
      {toasts.map((t) => (
        <Toast key={t.id} toast={t} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function Toast({ toast, onDismiss }: { toast: ToastMessage; onDismiss: (id: number) => void }) {
  // Mount animation: slide/fade in.
  const [shown, setShown] = useState(false);
  useEffect(() => {
    const id = requestAnimationFrame(() => setShown(true));
    return () => cancelAnimationFrame(id);
  }, []);

  const isError = toast.variant === "error";
  return (
    <div
      role="status"
      className={cn(
        "flex items-start gap-2 rounded-lg border bg-background p-3 text-sm shadow-lg transition-all duration-200",
        shown ? "translate-y-0 opacity-100" : "translate-y-2 opacity-0",
        isError ? "border-red-500/40" : "border-emerald-500/40"
      )}
      data-testid={`inbox-toast-${toast.variant}`}
    >
      {isError ? (
        <XCircle className="mt-0.5 h-4 w-4 shrink-0 text-red-500" />
      ) : (
        <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-emerald-500" />
      )}
      <p className="min-w-0 flex-1 break-words">{toast.text}</p>
      <button
        type="button"
        onClick={() => onDismiss(toast.id)}
        className="shrink-0 text-muted-foreground hover:text-foreground"
        aria-label="Dismiss"
      >
        <X className="h-4 w-4" />
      </button>
    </div>
  );
}
