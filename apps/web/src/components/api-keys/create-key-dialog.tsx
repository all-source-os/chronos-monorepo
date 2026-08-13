"use client";

import {
  Button,
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
  Input,
  Label,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, Check, Copy, Eye, EyeOff, Key, Loader2, X } from "lucide-react";
import { ApiKeyUsage } from "./api-key-usage";
import { useState } from "react";
import type { ApiKeyWithSecret } from "@/lib/api/client";

interface CreateKeyDialogProps {
  open: boolean;
  onClose: () => void;
  onCreateKey: (data: {
    name: string;
    description?: string;
    scopes: string[];
    expires_at?: string;
  }) => Promise<ApiKeyWithSecret | undefined>;
}

const AVAILABLE_SCOPES = [
  { id: "events:read", label: "Read Events", description: "Query and list events" },
  { id: "events:write", label: "Write Events", description: "Create and batch events" },
  { id: "queries:execute", label: "Execute Queries", description: "Run DSL queries" },
  { id: "projections:read", label: "Read Projections", description: "List and get projections" },
  { id: "projections:write", label: "Write Projections", description: "Create projections" },
  { id: "schemas:read", label: "Read Schemas", description: "List and get schemas" },
  { id: "schemas:write", label: "Write Schemas", description: "Register schemas" },
];

const EXPIRATION_OPTIONS = [
  { value: "", label: "Never expires" },
  { value: "30", label: "30 days" },
  { value: "90", label: "90 days" },
  { value: "365", label: "1 year" },
];

export function CreateKeyDialog({ open, onClose, onCreateKey }: CreateKeyDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  // Start empty, deliberately. Pre-selecting read+write made the first click on
  // the scope you wanted *remove* it: picking "Write Events" left ["events:read"]
  // and picking "Read Events" left ["events:write"], which reads as the dashboard
  // showing reversed labels. With nothing pre-selected, a click always adds.
  // Create is already disabled while no scope is chosen, so an explicit pick is
  // required — the right default for a credential anyway.
  const [selectedScopes, setSelectedScopes] = useState<string[]>([]);
  const [expiration, setExpiration] = useState("");
  const [isCreating, setIsCreating] = useState(false);
  const [createdKey, setCreatedKey] = useState<ApiKeyWithSecret | null>(null);
  const [showKey, setShowKey] = useState(true);
  const [copied, setCopied] = useState(false);

  const toggleScope = (scopeId: string) => {
    setSelectedScopes((prev) =>
      prev.includes(scopeId) ? prev.filter((s) => s !== scopeId) : [...prev, scopeId]
    );
  };

  const handleCreate = async () => {
    if (!name.trim() || selectedScopes.length === 0) return;

    setIsCreating(true);
    try {
      const expiresAt = expiration
        ? new Date(Date.now() + parseInt(expiration, 10) * 24 * 60 * 60 * 1000).toISOString()
        : undefined;

      const key = await onCreateKey({
        name: name.trim(),
        description: description.trim() || undefined,
        scopes: selectedScopes,
        expires_at: expiresAt,
      });

      if (key) {
        setCreatedKey(key);
      }
    } finally {
      setIsCreating(false);
    }
  };

  const handleCopy = async () => {
    if (createdKey) {
      await navigator.clipboard.writeText(createdKey.key);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  const handleClose = () => {
    setName("");
    setDescription("");
    setSelectedScopes([]);
    setExpiration("");
    setCreatedKey(null);
    onClose();
  };

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <button
        type="button"
        className="absolute inset-0 bg-background/80 backdrop-blur-sm"
        onClick={handleClose}
        aria-label="Close dialog"
      />

      {/* Dialog */}
      <Card className="relative z-10 w-full max-w-lg mx-4 max-h-[90vh] overflow-y-auto" onClick={(e) => e.stopPropagation()}>
        <CardHeader>
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center rounded-lg bg-primary/10">
              <Key className="h-5 w-5 text-primary" />
            </div>
            <div>
              <CardTitle>{createdKey ? "API Key Created" : "Create API Key"}</CardTitle>
              <CardDescription>
                {createdKey
                  ? "Save your key now - you won't see it again"
                  : "Generate a new API key for your applications"}
              </CardDescription>
            </div>
          </div>
          <Button
            variant="ghost"
            size="icon"
            onClick={handleClose}
            aria-label="Close"
            className="absolute right-4 top-4"
          >
            <X className="h-4 w-4" />
          </Button>
        </CardHeader>

        <CardContent>
          {createdKey ? (
            // Show created key
            <div className="space-y-4">
              <div className="rounded-lg border border-yellow-500/50 bg-yellow-500/10 p-4">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="h-5 w-5 shrink-0 text-yellow-600" />
                  <div>
                    <p className="font-medium text-yellow-600">Copy your key now</p>
                    <p className="text-sm text-yellow-600/80">
                      This key will only be shown once. Store it securely.
                    </p>
                  </div>
                </div>
              </div>

              <div>
                <Label>Your API Key</Label>
                <div className="mt-1.5 flex gap-2">
                  <div className="relative flex-1">
                    <Input
                      value={showKey ? createdKey.key : "•".repeat(createdKey.key.length)}
                      readOnly
                      className="font-mono pr-10"
                    />
                    <button
                      onClick={() => setShowKey(!showKey)}
                      className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                    >
                      {showKey ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                    </button>
                  </div>
                  <Button variant="outline" onClick={handleCopy}>
                    {copied ? (
                      <Check className="h-4 w-4 text-green-500" />
                    ) : (
                      <Copy className="h-4 w-4" />
                    )}
                  </Button>
                </div>
              </div>

              <div className="rounded-lg bg-muted p-4">
                <h4 className="mb-2 font-medium">Key Details</h4>
                <dl className="grid grid-cols-2 gap-2 text-sm">
                  <dt className="text-muted-foreground">Name</dt>
                  <dd>{createdKey.name}</dd>
                  <dt className="text-muted-foreground">Scopes</dt>
                  <dd>{createdKey.scopes.length} permissions</dd>
                  <dt className="text-muted-foreground">Expires</dt>
                  <dd>
                    {createdKey.expires_at
                      ? new Date(createdKey.expires_at).toLocaleDateString()
                      : "Never"}
                  </dd>
                </dl>
              </div>

              <ApiKeyUsage apiKey={createdKey.key} />
            </div>
          ) : (
            // Create form
            <div className="space-y-4">
              <div>
                <Label htmlFor="name">Name *</Label>
                <Input
                  id="name"
                  value={name}
                  onChange={(e) => setName(e.target.value)}
                  placeholder="e.g., Production Backend"
                  className="mt-1.5"
                />
              </div>

              <div>
                <Label htmlFor="description">Description</Label>
                <Input
                  id="description"
                  value={description}
                  onChange={(e) => setDescription(e.target.value)}
                  placeholder="What is this key for?"
                  className="mt-1.5"
                />
              </div>

              <div>
                <Label>Permissions *</Label>
                <div className="mt-1.5 grid grid-cols-2 gap-2">
                  {AVAILABLE_SCOPES.map((scope) => (
                    <button
                      key={scope.id}
                      type="button"
                      role="checkbox"
                      aria-checked={selectedScopes.includes(scope.id)}
                      onClick={() => toggleScope(scope.id)}
                      className={cn(
                        "flex items-start gap-2 rounded-lg border p-3 text-left transition-all",
                        selectedScopes.includes(scope.id)
                          ? "border-primary bg-primary/5"
                          : "border-border hover:border-primary/50"
                      )}
                    >
                      <div
                        className={cn(
                          "mt-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-sm border",
                          selectedScopes.includes(scope.id)
                            ? "border-primary bg-primary text-primary-foreground"
                            : "border-muted-foreground"
                        )}
                      >
                        {selectedScopes.includes(scope.id) && <Check className="h-3 w-3" />}
                      </div>
                      <div>
                        <p className="text-sm font-medium">{scope.label}</p>
                        <p className="text-xs text-muted-foreground">{scope.description}</p>
                      </div>
                    </button>
                  ))}
                </div>
              </div>

              <div>
                <Label htmlFor="expiration">Expiration</Label>
                <select
                  id="expiration"
                  value={expiration}
                  onChange={(e) => setExpiration(e.target.value)}
                  className="mt-1.5 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
                >
                  {EXPIRATION_OPTIONS.map((option) => (
                    <option key={option.value} value={option.value}>
                      {option.label}
                    </option>
                  ))}
                </select>
              </div>
            </div>
          )}
        </CardContent>

        <CardFooter className="flex justify-end gap-2">
          {createdKey ? (
            <Button onClick={handleClose}>Done</Button>
          ) : (
            <>
              <Button variant="outline" onClick={handleClose}>
                Cancel
              </Button>
              <Button
                data-testid="create-key-submit"
                onClick={handleCreate}
                disabled={!name.trim() || selectedScopes.length === 0 || isCreating}
              >
                {isCreating ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  <>
                    <Key className="mr-2 h-4 w-4" />
                    Create Key
                  </>
                )}
              </Button>
            </>
          )}
        </CardFooter>
      </Card>
    </div>
  );
}
