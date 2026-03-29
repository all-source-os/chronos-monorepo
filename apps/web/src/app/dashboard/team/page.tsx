"use client";

import { BlurFade, Button, Card, CardContent } from "@allsource/ui";
import { Plus, Users } from "lucide-react";
import { useState } from "react";
import { AgentKeysSection } from "@/components/team/agent-keys-section";
import { InviteMemberDialog } from "@/components/team/invite-member-dialog";
import { MemberTable } from "@/components/team/member-table";
import { useTeamMembers } from "@/hooks/use-team-members";

export default function TeamPage() {
  const { members, seatLimit, seatsUsed, isLoading, inviteMember, removeMember, updateRole } =
    useTeamMembers();
  const [showInviteDialog, setShowInviteDialog] = useState(false);
  const [confirmRemove, setConfirmRemove] = useState<string | null>(null);

  const handleInvite = async (email: string, role: string) => {
    await inviteMember({ email, role: role as "admin" | "member" | "viewer" });
  };

  const handleRemove = async (userId: string) => {
    if (confirmRemove !== userId) {
      setConfirmRemove(userId);
      return;
    }

    try {
      await removeMember(userId);
    } catch (error) {
      console.error("Failed to remove member:", error);
    }
    setConfirmRemove(null);
  };

  const handleUpdateRole = async (userId: string, role: string) => {
    try {
      await updateRole(userId, role);
    } catch (error) {
      console.error("Failed to update role:", error);
    }
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Team</h1>
            <p className="mt-1 text-muted-foreground">
              Manage your team members and control access to your workspace
            </p>
          </div>
          <Button onClick={() => setShowInviteDialog(true)}>
            <Plus className="mr-1.5 h-4 w-4" />
            Invite Member
          </Button>
        </div>
      </BlurFade>

      {/* Seat usage */}
      <BlurFade delay={0.2} inView>
        <Card className="border-primary/20 bg-primary/5">
          <CardContent className="flex items-start gap-4 p-4">
            <Users className="h-5 w-5 shrink-0 text-primary" />
            <div className="flex-1">
              <h3 className="font-medium">Team Seats</h3>
              <p className="text-sm text-muted-foreground">
                {seatsUsed} of {seatLimit} seats used. Team members share your tenant&apos;s event
                and query quota.
              </p>
              <div className="mt-2 h-2 w-full rounded-full bg-muted">
                <div
                  className="h-full rounded-full bg-primary transition-all"
                  style={{ width: `${Math.min(100, (seatsUsed / seatLimit) * 100)}%` }}
                />
              </div>
            </div>
          </CardContent>
        </Card>
      </BlurFade>

      {/* Members table */}
      <BlurFade delay={0.3} inView>
        <MemberTable
          members={members}
          isLoading={isLoading}
          onRemove={handleRemove}
          onUpdateRole={handleUpdateRole}
        />
      </BlurFade>

      {/* Agent Keys */}
      <BlurFade delay={0.4} inView>
        <AgentKeysSection />
      </BlurFade>

      {/* Invite dialog */}
      <InviteMemberDialog
        open={showInviteDialog}
        onClose={() => setShowInviteDialog(false)}
        onInvite={handleInvite}
        seatsUsed={seatsUsed}
        seatLimit={seatLimit}
      />

      {/* Remove confirmation */}
      {confirmRemove && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <button
            type="button"
            className="absolute inset-0 bg-background/80 backdrop-blur-sm"
            onClick={() => setConfirmRemove(null)}
            aria-label="Close confirmation dialog"
          />
          <Card className="relative z-10 w-full max-w-sm mx-4">
            <CardContent className="p-6">
              <Users className="mx-auto mb-4 h-12 w-12 text-destructive" />
              <h3 className="mb-2 text-center text-lg font-semibold">Remove Team Member?</h3>
              <p className="mb-6 text-center text-sm text-muted-foreground">
                This member will lose access to the workspace immediately. This action cannot be
                undone.
              </p>
              <div className="flex gap-2">
                <Button variant="outline" className="flex-1" onClick={() => setConfirmRemove(null)}>
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  className="flex-1"
                  onClick={() => handleRemove(confirmRemove)}
                >
                  Remove Member
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
