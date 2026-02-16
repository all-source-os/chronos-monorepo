"use client";

import { Button, Card, CardContent, Input, Label } from "@allsource/ui";
import { Loader2, Mail, X } from "lucide-react";
import { useState } from "react";

interface InviteMemberDialogProps {
  open: boolean;
  onClose: () => void;
  onInvite: (email: string, role: string) => Promise<void>;
  seatsUsed: number;
  seatLimit: number;
}

const roles = [
  {
    value: "admin",
    label: "Admin",
    description: "Can manage team members and all workspace settings",
  },
  {
    value: "member",
    label: "Member",
    description: "Can create events, projections, and API keys",
  },
  {
    value: "viewer",
    label: "Viewer",
    description: "Read-only access to events and analytics",
  },
];

export function InviteMemberDialog({
  open,
  onClose,
  onInvite,
  seatsUsed,
  seatLimit,
}: InviteMemberDialogProps) {
  const [email, setEmail] = useState("");
  const [role, setRole] = useState("member");
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);

  const seatsRemaining = seatLimit - seatsUsed;
  const atLimit = seatsRemaining <= 0;

  const handleSubmit = async () => {
    if (!email.trim() || atLimit) return;

    setIsSubmitting(true);
    setError(null);

    try {
      await onInvite(email.trim().toLowerCase(), role);
      setSuccess(true);
      setTimeout(() => {
        setEmail("");
        setRole("member");
        setSuccess(false);
        onClose();
      }, 1500);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to send invitation");
    } finally {
      setIsSubmitting(false);
    }
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      <button
        type="button"
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={onClose}
        aria-label="Close dialog"
      />
      <Card className="relative z-10 w-full max-w-md mx-4">
        <CardContent className="p-6">
          <div className="mb-6 flex items-center justify-between">
            <div>
              <h2 className="text-lg font-semibold">Invite Team Member</h2>
              <p className="text-sm text-muted-foreground">
                {seatsRemaining} of {seatLimit} seats remaining
              </p>
            </div>
            <Button variant="ghost" size="icon" onClick={onClose} className="h-8 w-8">
              <X className="h-4 w-4" />
            </Button>
          </div>

          {success ? (
            <div className="flex flex-col items-center py-8">
              <div className="mb-4 flex h-16 w-16 items-center justify-center rounded-full bg-green-500/10">
                <Mail className="h-8 w-8 text-green-500" />
              </div>
              <h3 className="mb-1 text-lg font-medium">Invitation Sent</h3>
              <p className="text-center text-sm text-muted-foreground">
                An invitation email has been sent to {email}
              </p>
            </div>
          ) : (
            <div className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="invite-email">Email address</Label>
                <Input
                  id="invite-email"
                  type="email"
                  placeholder="colleague@company.com"
                  value={email}
                  onChange={(e) => setEmail(e.target.value)}
                  disabled={isSubmitting || atLimit}
                />
              </div>

              <div className="space-y-2">
                <Label>Role</Label>
                <div className="space-y-2">
                  {roles.map((r) => (
                    <button
                      key={r.value}
                      type="button"
                      className={`flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors ${
                        role === r.value
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-muted-foreground/30"
                      }`}
                      onClick={() => setRole(r.value)}
                      disabled={isSubmitting}
                    >
                      <div
                        className={`mt-0.5 h-4 w-4 rounded-full border-2 ${
                          role === r.value
                            ? "border-primary bg-primary"
                            : "border-muted-foreground/30"
                        }`}
                      />
                      <div>
                        <p className="text-sm font-medium">{r.label}</p>
                        <p className="text-xs text-muted-foreground">{r.description}</p>
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              {atLimit && (
                <div className="rounded-lg border border-yellow-500/20 bg-yellow-500/10 p-3">
                  <p className="text-sm text-yellow-600 dark:text-yellow-400">
                    You&apos;ve reached your seat limit. Upgrade your plan to invite more members.
                  </p>
                </div>
              )}

              {error && (
                <div className="rounded-lg border border-destructive/20 bg-destructive/10 p-3">
                  <p className="text-sm text-destructive">{error}</p>
                </div>
              )}

              <div className="flex gap-2 pt-2">
                <Button variant="outline" className="flex-1" onClick={onClose}>
                  Cancel
                </Button>
                <Button
                  className="flex-1"
                  onClick={handleSubmit}
                  disabled={!email.trim() || isSubmitting || atLimit}
                >
                  {isSubmitting ? (
                    <>
                      <Loader2 className="mr-1.5 h-4 w-4 animate-spin" />
                      Sending...
                    </>
                  ) : (
                    "Send Invitation"
                  )}
                </Button>
              </div>
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
