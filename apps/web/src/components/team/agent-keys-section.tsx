"use client";

import { Badge, Button, Card, CardContent } from "@allsource/ui";
import { Bot, Check, Copy, KeyRound, Loader2, Plus, Trash2, X } from "lucide-react";
import { useState } from "react";
import useSWR, { mutate } from "swr";
import type { AgentKey, AgentKeyCreated } from "@/lib/api/client";
import { apiClient } from "@/lib/api/client";

const SWR_KEY = "/api/team/agent-keys";

function fetcher() {
  return apiClient.listAgentKeys().then((r) => {
    if (r.error) throw new Error(r.error.message);
    return r.data;
  });
}

export function AgentKeysSection() {
  const { data, isLoading } = useSWR(SWR_KEY, fetcher, { revalidateOnFocus: false });
  const keys: AgentKey[] = data?.agent_keys ?? [];

  const [creating, setCreating] = useState(false);
  const [newName, setNewName] = useState("");
  const [createError, setCreateError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  // Holds the newly-created key value for one-time display
  const [revealed, setRevealed] = useState<AgentKeyCreated | null>(null);
  const [copied, setCopied] = useState(false);

  const [revokingName, setRevokingName] = useState<string | null>(null);

  const handleCreate = async () => {
    const name = newName.trim();
    if (!name) return;
    setIsSubmitting(true);
    setCreateError(null);
    const resp = await apiClient.createAgentKey(name);
    setIsSubmitting(false);
    if (resp.error) {
      setCreateError(resp.error.message);
      return;
    }
    setRevealed(resp.data ?? null);
    setNewName("");
    setCreating(false);
    mutate(SWR_KEY);
  };

  const handleCopy = async () => {
    if (!revealed) return;
    await navigator.clipboard.writeText(revealed.key);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  const handleRevoke = async (name: string) => {
    setRevokingName(name);
    await apiClient.revokeAgentKey(name);
    setRevokingName(null);
    mutate(SWR_KEY);
  };

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between">
        <div>
          <h3 className="font-semibold">Agent Keys</h3>
          <p className="text-sm text-muted-foreground">
            Provision <code className="rounded bg-muted px-1 text-xs">ask_...</code> keys so AI
            agents can sync chronis tasks to your team&apos;s event store.
          </p>
        </div>
        {!creating && (
          <Button size="sm" variant="outline" onClick={() => setCreating(true)}>
            <Plus className="mr-1.5 h-3.5 w-3.5" />
            New Key
          </Button>
        )}
      </div>

      {/* Create form */}
      {creating && (
        <Card className="border-dashed">
          <CardContent className="flex items-center gap-3 p-4">
            <KeyRound className="h-4 w-4 shrink-0 text-muted-foreground" />
            <input
              autoFocus
              className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
              placeholder="Key name, e.g. ralph-tui-prod"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter") handleCreate();
                if (e.key === "Escape") setCreating(false);
              }}
            />
            <Button size="sm" onClick={handleCreate} disabled={isSubmitting || !newName.trim()}>
              {isSubmitting ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "Create"}
            </Button>
            <Button size="sm" variant="ghost" onClick={() => setCreating(false)}>
              <X className="h-3.5 w-3.5" />
            </Button>
          </CardContent>
          {createError && (
            <p className="px-4 pb-3 text-xs text-destructive">{createError}</p>
          )}
        </Card>
      )}

      {/* One-time key reveal */}
      {revealed && (
        <Card className="border-green-500/30 bg-green-500/5">
          <CardContent className="space-y-3 p-4">
            <div className="flex items-center gap-2 text-sm font-medium text-green-600 dark:text-green-400">
              <Check className="h-4 w-4" />
              Key created — copy it now. It will not be shown again.
            </div>
            <div className="flex items-center gap-2 rounded-md border border-border bg-background px-3 py-2">
              <code className="flex-1 truncate font-mono text-xs">{revealed.key}</code>
              <Button size="sm" variant="ghost" className="h-7 shrink-0 px-2" onClick={handleCopy}>
                {copied ? (
                  <Check className="h-3.5 w-3.5 text-green-500" />
                ) : (
                  <Copy className="h-3.5 w-3.5" />
                )}
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Add to <code className="rounded bg-muted px-1">.chronis/config.toml</code>:
            </p>
            <pre className="rounded-md bg-muted p-3 font-mono text-xs leading-relaxed">
              {`mode = "remote"\n[sync]\nremote_url = "https://core.allsource.io"\napi_key    = "${revealed.key}"`}
            </pre>
            <Button size="sm" variant="outline" onClick={() => setRevealed(null)}>
              Done
            </Button>
          </CardContent>
        </Card>
      )}

      {/* Key list */}
      {isLoading ? (
        <Card>
          <CardContent className="p-4">
            <div className="space-y-3">
              {[1, 2].map((i) => (
                <div key={i} className="flex items-center gap-3">
                  <div className="h-4 w-4 animate-pulse rounded bg-muted" />
                  <div className="h-4 flex-1 animate-pulse rounded bg-muted" />
                  <div className="h-4 w-20 animate-pulse rounded bg-muted" />
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      ) : keys.length === 0 ? (
        <Card>
          <CardContent className="flex flex-col items-center justify-center p-8 text-center">
            <Bot className="mb-3 h-10 w-10 text-muted-foreground" />
            <p className="text-sm font-medium">No agent keys yet</p>
            <p className="mt-1 text-xs text-muted-foreground">
              Create a key to let AI agents sync chronis tasks to your workspace.
            </p>
          </CardContent>
        </Card>
      ) : (
        <Card>
          <CardContent className="p-0">
            <table className="w-full">
              <thead>
                <tr className="border-b border-border">
                  <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                    Name
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                    Key ID
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium uppercase tracking-wider text-muted-foreground">
                    Created
                  </th>
                  <th className="px-6 py-3 text-right text-xs font-medium uppercase tracking-wider text-muted-foreground">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border">
                {keys.map((key) => (
                  <tr key={key.key_id} className="hover:bg-muted/50 transition-colors">
                    <td className="px-6 py-4">
                      <div className="flex items-center gap-2">
                        <Bot className="h-4 w-4 shrink-0 text-muted-foreground" />
                        <span className="text-sm font-medium">{key.name}</span>
                      </div>
                    </td>
                    <td className="px-6 py-4">
                      <Badge variant="outline" className="font-mono text-xs">
                        {key.key_id.slice(0, 8)}…
                      </Badge>
                    </td>
                    <td className="px-6 py-4 text-sm text-muted-foreground">
                      {new Date(key.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-6 py-4 text-right">
                      <Button
                        size="sm"
                        variant="ghost"
                        className="h-7 px-2 text-destructive hover:text-destructive"
                        onClick={() => handleRevoke(key.name)}
                        disabled={revokingName === key.name}
                      >
                        {revokingName === key.name ? (
                          <Loader2 className="h-3.5 w-3.5 animate-spin" />
                        ) : (
                          <Trash2 className="h-3.5 w-3.5" />
                        )}
                      </Button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
