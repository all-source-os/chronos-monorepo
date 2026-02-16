"use client";

import { Badge, Button, Card, CardContent } from "@allsource/ui";
import {
  ChevronDown,
  Crown,
  Loader2,
  MoreHorizontal,
  Shield,
  Trash2,
  User,
  UserCheck,
} from "lucide-react";
import { useState } from "react";
import type { TeamMember } from "@/lib/api/client";

interface MemberTableProps {
  members: TeamMember[];
  isLoading: boolean;
  onRemove: (userId: string) => void;
  onUpdateRole: (userId: string, role: string) => void;
}

const roleIcons: Record<string, typeof User> = {
  owner: Crown,
  admin: Shield,
  member: UserCheck,
  viewer: User,
};

const roleBadgeVariants: Record<string, string> = {
  owner: "bg-amber-500/10 text-amber-500 border-amber-500/20",
  admin: "bg-blue-500/10 text-blue-500 border-blue-500/20",
  member: "bg-green-500/10 text-green-500 border-green-500/20",
  viewer: "bg-muted text-muted-foreground border-border",
};

export function MemberTable({ members, isLoading, onRemove, onUpdateRole }: MemberTableProps) {
  const [openMenu, setOpenMenu] = useState<string | null>(null);
  const [roleMenu, setRoleMenu] = useState<string | null>(null);
  const [removingId, setRemovingId] = useState<string | null>(null);

  if (isLoading) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="space-y-4">
            {[1, 2, 3].map((i) => (
              <div key={i} className="flex items-center gap-4">
                <div className="h-10 w-10 animate-pulse rounded-full bg-muted" />
                <div className="flex-1 space-y-2">
                  <div className="h-4 w-32 animate-pulse rounded bg-muted" />
                  <div className="h-3 w-48 animate-pulse rounded bg-muted" />
                </div>
                <div className="h-6 w-16 animate-pulse rounded bg-muted" />
              </div>
            ))}
          </div>
        </CardContent>
      </Card>
    );
  }

  if (members.length === 0) {
    return (
      <Card>
        <CardContent className="flex flex-col items-center justify-center p-12 text-center">
          <User className="mb-4 h-12 w-12 text-muted-foreground" />
          <h3 className="mb-1 text-lg font-medium">No team members</h3>
          <p className="text-sm text-muted-foreground">
            Invite team members to collaborate on your workspace.
          </p>
        </CardContent>
      </Card>
    );
  }

  const handleRemove = async (userId: string) => {
    setRemovingId(userId);
    try {
      onRemove(userId);
    } finally {
      setRemovingId(null);
      setOpenMenu(null);
    }
  };

  return (
    <Card>
      <CardContent className="p-0">
        <div className="overflow-x-auto">
          <table className="w-full">
            <thead>
              <tr className="border-b border-border">
                <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Member
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Role
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Status
                </th>
                <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Joined
                </th>
                <th className="px-6 py-3 text-right text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  Actions
                </th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border">
              {members.map((member) => {
                const RoleIcon = roleIcons[member.role] || User;
                const isOwner = member.role === "owner";

                return (
                  <tr key={member.id} className="hover:bg-muted/50 transition-colors">
                    <td className="px-6 py-4">
                      <div className="flex items-center gap-3">
                        <div className="flex h-10 w-10 items-center justify-center rounded-full bg-primary/10 text-sm font-medium text-primary">
                          {member.name.charAt(0).toUpperCase()}
                        </div>
                        <div>
                          <p className="text-sm font-medium">{member.name}</p>
                          <p className="text-xs text-muted-foreground">{member.email}</p>
                        </div>
                      </div>
                    </td>
                    <td className="px-6 py-4">
                      <div className="relative">
                        {isOwner ? (
                          <Badge
                            variant="outline"
                            className={`gap-1 ${roleBadgeVariants[member.role]}`}
                          >
                            <RoleIcon className="h-3 w-3" />
                            {member.role}
                          </Badge>
                        ) : (
                          <button
                            type="button"
                            className={`inline-flex items-center gap-1 rounded-full border px-2.5 py-0.5 text-xs font-semibold transition-colors hover:opacity-80 ${roleBadgeVariants[member.role]}`}
                            onClick={() => setRoleMenu(roleMenu === member.id ? null : member.id)}
                          >
                            <RoleIcon className="h-3 w-3" />
                            {member.role}
                            <ChevronDown className="h-3 w-3" />
                          </button>
                        )}
                        {roleMenu === member.id && !isOwner && (
                          <>
                            <button
                              type="button"
                              className="fixed inset-0 z-40"
                              onClick={() => setRoleMenu(null)}
                              aria-label="Close role menu"
                            />
                            <div className="absolute left-0 top-full z-50 mt-1 w-32 rounded-md border border-border bg-background shadow-lg">
                              {["admin", "member", "viewer"].map((role) => (
                                <button
                                  key={role}
                                  type="button"
                                  className="flex w-full items-center gap-2 px-3 py-2 text-xs capitalize hover:bg-muted"
                                  onClick={() => {
                                    onUpdateRole(member.user_id, role);
                                    setRoleMenu(null);
                                  }}
                                >
                                  {role}
                                  {role === member.role && (
                                    <span className="ml-auto text-primary">&#10003;</span>
                                  )}
                                </button>
                              ))}
                            </div>
                          </>
                        )}
                      </div>
                    </td>
                    <td className="px-6 py-4">
                      <Badge
                        variant="outline"
                        className={
                          member.status === "active"
                            ? "bg-green-500/10 text-green-500 border-green-500/20"
                            : "bg-yellow-500/10 text-yellow-500 border-yellow-500/20"
                        }
                      >
                        {member.status}
                      </Badge>
                    </td>
                    <td className="px-6 py-4 text-sm text-muted-foreground">
                      {new Date(member.joined_at).toLocaleDateString()}
                    </td>
                    <td className="px-6 py-4 text-right">
                      {isOwner ? (
                        <span className="text-xs text-muted-foreground">-</span>
                      ) : (
                        <div className="relative inline-block">
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
                            onClick={() => setOpenMenu(openMenu === member.id ? null : member.id)}
                          >
                            <MoreHorizontal className="h-4 w-4" />
                          </Button>
                          {openMenu === member.id && (
                            <>
                              <button
                                type="button"
                                className="fixed inset-0 z-40"
                                onClick={() => setOpenMenu(null)}
                                aria-label="Close actions menu"
                              />
                              <div className="absolute right-0 top-full z-50 mt-1 w-40 rounded-md border border-border bg-background shadow-lg">
                                <button
                                  type="button"
                                  className="flex w-full items-center gap-2 px-3 py-2 text-xs text-destructive hover:bg-muted"
                                  onClick={() => handleRemove(member.user_id)}
                                  disabled={removingId === member.user_id}
                                >
                                  {removingId === member.user_id ? (
                                    <Loader2 className="h-3 w-3 animate-spin" />
                                  ) : (
                                    <Trash2 className="h-3 w-3" />
                                  )}
                                  Remove member
                                </button>
                              </div>
                            </>
                          )}
                        </div>
                      )}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </CardContent>
    </Card>
  );
}
