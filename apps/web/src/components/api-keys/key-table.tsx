"use client";

import { Badge, Button } from "@allsource/ui";
import { Check, Clock, Copy, Key, MoreHorizontal, RefreshCw, Trash2 } from "lucide-react";
import { type MouseEvent, useState } from "react";
import type { ApiKey } from "@/lib/api/client";

interface KeyTableProps {
  keys: ApiKey[];
  isLoading?: boolean;
  onRotate: (id: string) => void;
  onRevoke: (id: string) => void;
}

function KeyRow({
  apiKey,
  onRotate,
  onRevoke,
}: {
  apiKey: ApiKey;
  onRotate: () => void;
  onRevoke: () => void;
}) {
  const [showMenu, setShowMenu] = useState(false);
  const [menuPos, setMenuPos] = useState<{ top: number; right: number } | null>(null);
  const [copied, setCopied] = useState(false);

  // The full secret is write-once (returned only at create/rotate via
  // ApiKeyWithSecret); the list carries just key_prefix. Copy the clean prefix
  // — no trailing "..." — so what lands on the clipboard is a real value.
  const copyPrefix = async () => {
    await navigator.clipboard.writeText(apiKey.key_prefix);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  };

  // Position the menu with fixed coords from the trigger so it escapes the
  // table wrapper's `overflow-hidden` (which clips an absolutely-positioned
  // dropdown regardless of z-index).
  const toggleMenu = (e: MouseEvent<HTMLButtonElement>) => {
    if (showMenu) {
      setShowMenu(false);
      return;
    }
    const r = e.currentTarget.getBoundingClientRect();
    setMenuPos({ top: r.bottom + 4, right: window.innerWidth - r.right });
    setShowMenu(true);
  };

  const formatDate = (dateStr: string | null) => {
    if (!dateStr) return "Never";
    return new Date(dateStr).toLocaleDateString();
  };

  const isExpired = apiKey.expires_at && new Date(apiKey.expires_at) < new Date();

  return (
    <tr className="border-b border-border transition-colors hover:bg-muted/50">
      <td className="px-4 py-3">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-primary/10">
            <Key className="h-4 w-4 text-primary" />
          </div>
          <div>
            <p className="font-medium">{apiKey.name}</p>
            {apiKey.description && (
              <p className="text-xs text-muted-foreground">{apiKey.description}</p>
            )}
          </div>
        </div>
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-2">
          <code className="rounded bg-muted px-2 py-1 font-mono text-xs">
            {apiKey.key_prefix}...
          </code>
          <Button variant="ghost" size="icon" className="h-6 w-6" onClick={copyPrefix}>
            {copied ? <Check className="h-3 w-3 text-green-500" /> : <Copy className="h-3 w-3" />}
          </Button>
        </div>
      </td>
      <td className="px-4 py-3">
        <div className="flex flex-wrap gap-1">
          {apiKey.scopes.slice(0, 3).map((scope) => (
            <Badge key={scope} variant="secondary" className="text-xs">
              {scope}
            </Badge>
          ))}
          {apiKey.scopes.length > 3 && (
            <Badge variant="secondary" className="text-xs">
              +{apiKey.scopes.length - 3}
            </Badge>
          )}
        </div>
      </td>
      <td className="px-4 py-3">
        <div className="flex items-center gap-1.5 text-sm text-muted-foreground">
          <Clock className="h-3.5 w-3.5" />
          {formatDate(apiKey.last_used_at)}
        </div>
      </td>
      <td className="px-4 py-3">
        <span className="text-sm text-muted-foreground">{formatDate(apiKey.created_at)}</span>
      </td>
      <td className="px-4 py-3">
        {isExpired ? (
          <Badge variant="destructive" className="text-xs">
            Expired
          </Badge>
        ) : apiKey.expires_at ? (
          <span className="text-sm text-muted-foreground">{formatDate(apiKey.expires_at)}</span>
        ) : (
          <span className="text-sm text-muted-foreground">Never</span>
        )}
      </td>
      <td className="px-4 py-3">
        <div className="relative">
          <Button variant="ghost" size="icon" className="h-8 w-8" onClick={toggleMenu}>
            <MoreHorizontal className="h-4 w-4" />
          </Button>

          {showMenu && menuPos && (
            <>
              <div className="fixed inset-0 z-40" onClick={() => setShowMenu(false)} />
              <div
                className="fixed z-50 w-40 rounded-lg border border-border bg-popover p-1 shadow-lg"
                style={{ top: menuPos.top, right: menuPos.right }}
              >
                <button
                  onClick={() => {
                    setShowMenu(false);
                    onRotate();
                  }}
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm hover:bg-muted"
                >
                  <RefreshCw className="h-4 w-4" />
                  Rotate Key
                </button>
                <button
                  onClick={() => {
                    setShowMenu(false);
                    onRevoke();
                  }}
                  className="flex w-full items-center gap-2 rounded-md px-3 py-2 text-sm text-destructive hover:bg-destructive/10"
                >
                  <Trash2 className="h-4 w-4" />
                  Revoke Key
                </button>
              </div>
            </>
          )}
        </div>
      </td>
    </tr>
  );
}

export function KeyTable({ keys, isLoading, onRotate, onRevoke }: KeyTableProps) {
  if (isLoading) {
    return (
      <div className="overflow-hidden rounded-lg border border-border">
        <table className="w-full">
          <thead className="border-b border-border bg-muted/50">
            <tr>
              <th className="px-4 py-3 text-left text-sm font-medium">Name</th>
              <th className="px-4 py-3 text-left text-sm font-medium">Key</th>
              <th className="px-4 py-3 text-left text-sm font-medium">Scopes</th>
              <th className="px-4 py-3 text-left text-sm font-medium">Last Used</th>
              <th className="px-4 py-3 text-left text-sm font-medium">Created</th>
              <th className="px-4 py-3 text-left text-sm font-medium">Expires</th>
              <th className="px-4 py-3 text-left text-sm font-medium"></th>
            </tr>
          </thead>
          <tbody>
            {Array.from({ length: 3 }).map((_, i) => (
              <tr key={i} className="border-b border-border">
                <td className="px-4 py-3">
                  <div className="h-8 w-32 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-6 w-24 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-6 w-20 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-5 w-16 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-5 w-16 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-5 w-16 animate-pulse rounded bg-muted" />
                </td>
                <td className="px-4 py-3">
                  <div className="h-8 w-8 animate-pulse rounded bg-muted" />
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    );
  }

  if (keys.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center rounded-lg border border-dashed border-border py-12 text-center">
        <Key className="mb-4 h-12 w-12 text-muted-foreground/50" />
        <p className="text-lg font-medium">No API keys yet</p>
        <p className="text-sm text-muted-foreground">
          Create your first API key to connect your applications
        </p>
      </div>
    );
  }

  return (
    <div className="overflow-hidden rounded-lg border border-border">
      <table className="w-full">
        <thead className="border-b border-border bg-muted/50">
          <tr>
            <th className="px-4 py-3 text-left text-sm font-medium">Name</th>
            <th className="px-4 py-3 text-left text-sm font-medium">Key</th>
            <th className="px-4 py-3 text-left text-sm font-medium">Scopes</th>
            <th className="px-4 py-3 text-left text-sm font-medium">Last Used</th>
            <th className="px-4 py-3 text-left text-sm font-medium">Created</th>
            <th className="px-4 py-3 text-left text-sm font-medium">Expires</th>
            <th className="px-4 py-3 text-left text-sm font-medium"></th>
          </tr>
        </thead>
        <tbody>
          {keys.map((key) => (
            <KeyRow
              key={key.id}
              apiKey={key}
              onRotate={() => onRotate(key.id)}
              onRevoke={() => onRevoke(key.id)}
            />
          ))}
        </tbody>
      </table>
    </div>
  );
}
