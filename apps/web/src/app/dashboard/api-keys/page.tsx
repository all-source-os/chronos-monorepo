"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { AlertTriangle, Key, Plus, Shield } from "lucide-react";
import { useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { CreateKeyDialog } from "@/components/api-keys/create-key-dialog";
import { KeyTable } from "@/components/api-keys/key-table";
import { LoadError } from "@/components/dashboard/load-error";
import { FadeIn } from "@/components/ui/fade-in";
import { useApiKeys } from "@/hooks/use-api-keys";
import type { ApiKeyWithSecret } from "@/lib/api/client";

export default function ApiKeysPage() {
  const searchParams = useSearchParams();
  const { keys, isLoading, error, createKey, rotateKey, revokeKey, refresh } = useApiKeys();
  const [showCreateDialog, setShowCreateDialog] = useState(false);
  const [confirmRevoke, setConfirmRevoke] = useState<string | null>(null);

  // Check URL for action param
  useEffect(() => {
    if (searchParams.get("action") === "create") {
      setShowCreateDialog(true);
    }
  }, [searchParams]);

  const handleCreateKey = async (data: {
    name: string;
    description?: string;
    scopes: string[];
    expires_at?: string;
  }): Promise<ApiKeyWithSecret | undefined> => {
    try {
      return await createKey(data);
    } catch (error) {
      console.error("Failed to create API key:", error);
      return undefined;
    }
  };

  const handleRotate = async (id: string) => {
    try {
      await rotateKey(id);
    } catch (error) {
      console.error("Failed to rotate API key:", error);
    }
  };

  const handleRevoke = async (id: string) => {
    if (confirmRevoke !== id) {
      setConfirmRevoke(id);
      return;
    }

    try {
      await revokeKey(id);
    } catch (error) {
      console.error("Failed to revoke API key:", error);
    }
    setConfirmRevoke(null);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <FadeIn delay={0.1} inView>
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
      </FadeIn>

      {/* Security notice */}
      <FadeIn delay={0.2} inView>
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
      </FadeIn>

      {/* Keys table */}
      <FadeIn delay={0.3} inView>
        {error ? (
          <LoadError title="API keys could not be loaded" message={error} onRetry={refresh} />
        ) : (
          <KeyTable
            keys={keys}
            isLoading={isLoading}
            onRotate={handleRotate}
            onRevoke={handleRevoke}
          />
        )}
      </FadeIn>

      {/* Quick tips */}
      <FadeIn delay={0.4} inView>
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
      </FadeIn>

      {/* Create dialog */}
      <CreateKeyDialog
        open={showCreateDialog}
        onClose={() => setShowCreateDialog(false)}
        onCreateKey={handleCreateKey}
      />

      {/* Revoke confirmation */}
      {confirmRevoke && (
        <div className="fixed inset-0 z-50 flex items-center justify-center">
          <button
            type="button"
            className="absolute inset-0 bg-background/80 backdrop-blur-sm"
            onClick={() => setConfirmRevoke(null)}
            aria-label="Cancel API key revocation"
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
