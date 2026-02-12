"use client";

import { BlurFade, Button, Card, CardContent } from "@allsource/ui";
import { AlertTriangle, Key, Plus, Shield } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { CreateKeyDialog } from "@/components/api-keys/create-key-dialog";
import { KeyTable } from "@/components/api-keys/key-table";
import type { ApiKey, ApiKeyWithSecret } from "@/lib/api/client";

// Demo keys for display
const DEMO_KEYS: ApiKey[] = [
  {
    id: "key-1",
    name: "Production Backend",
    description: "Main backend service",
    key_prefix: "qs_live_prod",
    scopes: ["events:read", "events:write", "queries:execute"],
    last_used_at: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
    expires_at: null,
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 30).toISOString(),
  },
  {
    id: "key-2",
    name: "Analytics Service",
    description: "Read-only analytics",
    key_prefix: "qs_live_analytics",
    scopes: ["events:read", "queries:execute"],
    last_used_at: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
    expires_at: new Date(Date.now() + 1000 * 60 * 60 * 24 * 60).toISOString(),
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 15).toISOString(),
  },
  {
    id: "key-3",
    name: "Development",
    description: "Local development key",
    key_prefix: "qs_test_dev",
    scopes: [
      "events:read",
      "events:write",
      "queries:execute",
      "projections:read",
      "projections:write",
    ],
    last_used_at: null,
    expires_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 5).toISOString(), // Expired
    created_at: new Date(Date.now() - 1000 * 60 * 60 * 24 * 60).toISOString(),
  },
];

export default function ApiKeysPage() {
  const searchParams = useSearchParams();
  const [keys, setKeys] = useState<ApiKey[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState<string | null>(null);

  // Check URL for action param
  useEffect(() => {
    if (searchParams.get("action") === "create") {
      setShowCreateDialog(true);
    }
  }, [searchParams]);

  // Simulate loading
  useEffect(() => {
    setTimeout(() => {
      setKeys(DEMO_KEYS);
      setIsLoading(false);
    }, 500);
  }, []);

  const handleCreateKey = async (data: {
    name: string;
    description?: string;
    scopes: string[];
    expires_at?: string;
  }): Promise<ApiKeyWithSecret | undefined> => {
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1000));

    const newKey: ApiKeyWithSecret = {
      id: `key-${Date.now()}`,
      name: data.name,
      description: data.description || null,
      key_prefix: `qs_live_${Math.random().toString(36).slice(2, 8)}`,
      key: `qs_live_${Math.random().toString(36).slice(2)}${Math.random().toString(36).slice(2)}`,
      scopes: data.scopes,
      last_used_at: null,
      expires_at: data.expires_at || null,
      created_at: new Date().toISOString(),
    };

    setKeys((prev) => [newKey, ...prev]);
    return newKey;
  };

  const handleRotate = async (id: string) => {
    // Simulate rotation
    await new Promise((resolve) => setTimeout(resolve, 500));
    setKeys((prev) =>
      prev.map((key) =>
        key.id === id
          ? {
              ...key,
              key_prefix: `qs_live_${Math.random().toString(36).slice(2, 8)}`,
              created_at: new Date().toISOString(),
            }
          : key
      )
    );
  };

  const handleRevoke = async (id: string) => {
    if (confirmRevoke !== id) {
      setConfirmRevoke(id);
      return;
    }

    // Simulate revocation
    await new Promise((resolve) => setTimeout(resolve, 500));
    setKeys((prev) => prev.filter((key) => key.id !== id));
    setConfirmRevoke(null);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">API Keys</h1>
            <p className="mt-1 text-muted-foreground">
              Manage API keys for authenticating your applications
            </p>
          </div>
          <Button onClick={() => setShowCreateDialog(true)}>
            <Plus className="mr-1.5 h-4 w-4" />
            Create Key
          </Button>
        </div>
      </BlurFade>

      {/* Security notice */}
      <BlurFade delay={0.2} inView>
        <Card className="border-primary/20 bg-primary/5">
          <CardContent className="flex items-start gap-4 p-4">
            <Shield className="h-5 w-5 shrink-0 text-primary" />
            <div>
              <h3 className="font-medium">Keep your keys secure</h3>
              <p className="text-sm text-muted-foreground">
                API keys grant access to your AllSource account. Never share them publicly or commit
                them to version control. Use environment variables instead.
              </p>
            </div>
          </CardContent>
        </Card>
      </BlurFade>

      {/* Keys table */}
      <BlurFade delay={0.3} inView>
        <KeyTable
          keys={keys}
          isLoading={isLoading}
          onRotate={handleRotate}
          onRevoke={handleRevoke}
        />
      </BlurFade>

      {/* Quick tips */}
      <BlurFade delay={0.4} inView>
        <div className="grid gap-4 sm:grid-cols-3">
          <Card>
            <CardContent className="p-4">
              <Key className="mb-2 h-5 w-5 text-muted-foreground" />
              <h3 className="font-medium">Scope Permissions</h3>
              <p className="text-sm text-muted-foreground">
                Only grant the minimum permissions needed for each key.
              </p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <Shield className="mb-2 h-5 w-5 text-muted-foreground" />
              <h3 className="font-medium">Rotate Regularly</h3>
              <p className="text-sm text-muted-foreground">
                Rotate keys periodically to reduce risk of compromised credentials.
              </p>
            </CardContent>
          </Card>
          <Card>
            <CardContent className="p-4">
              <AlertTriangle className="mb-2 h-5 w-5 text-muted-foreground" />
              <h3 className="font-medium">Monitor Usage</h3>
              <p className="text-sm text-muted-foreground">
                Check "Last Used" to identify unused keys that can be revoked.
              </p>
            </CardContent>
          </Card>
        </div>
      </BlurFade>

      {/* Create dialog */}
      <CreateKeyDialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onCreateKey={handleCreateKey}
      />

      {/* Revoke confirmation */}
      {confirmRevoke && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <div
            className="absolute inset-0 bg-background/80 backdrop-blur-sm"
            onClick={() => setConfirmRevoke(null)}
          />
          <Card className="relative z-10 w-full max-w-sm mx-4">
            <CardContent className="p-6">
              <AlertTriangle className="mx-auto mb-4 h-12 w-12 text-destructive" />
              <h3 className="mb-2 text-center text-lg font-semibold">Revoke API Key?</h3>
              <p className="mb-6 text-center text-sm text-muted-foreground">
                This action cannot be undone. Any applications using this key will stop working
                immediately.
              </p>
              <div className="flex gap-2">
                <Button variant="outline" className="flex-1" onClick={() => setConfirmRevoke(null)}>
                  Cancel
                </Button>
                <Button
                  variant="destructive"
                  className="flex-1"
                  onClick={() => handleRevoke(confirmRevoke)}
                >
                  Revoke Key
                </Button>
              </div>
            </CardContent>
          </Card>
        </div>
      )}
    </div>
  );
}
