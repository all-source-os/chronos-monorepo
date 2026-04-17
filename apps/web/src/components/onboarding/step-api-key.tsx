"use client";

import { Button, Card, CardContent, Input, Label } from "@allsource/ui";
import { AlertTriangle, ArrowRight, Check, Copy, Eye, EyeOff, Key, Loader2 } from "lucide-react";
import { useState } from "react";
import { ApiKeyUsage } from "@/components/api-keys/api-key-usage";
import { useOnboarding } from "@/hooks/use-onboarding";
import { apiClient } from "@/lib/api/client";

interface StepApiKeyProps {
  onNext: () => void;
}

export function StepApiKey({ onNext }: StepApiKeyProps) {
  const { setApiKeyCreated } = useOnboarding();
  const [keyName, setKeyName] = useState("My First API Key");
  const [isCreating, setIsCreating] = useState(false);
  const [apiKey, setApiKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [showKey, setShowKey] = useState(true);
  const [copied, setCopied] = useState(false);

  const handleCreate = async () => {
    setIsCreating(true);
    setError(null);
    try {
      const response = await apiClient.createApiKey({
        name: keyName.trim(),
        scopes: ["events:read", "events:write", "queries:execute", "projections:read"],
      });
      if (response.error) {
        setError(response.error.message);
      } else if (response.data?.key) {
        setApiKey(response.data.key);
        setApiKeyCreated(true);
      }
    } catch {
      setError("Failed to create API key. Please try again.");
    } finally {
      setIsCreating(false);
    }
  };

  const handleCopy = async () => {
    if (apiKey) {
      await navigator.clipboard.writeText(apiKey);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-8 text-center">
        <h2 className="mb-2 text-2xl font-bold">Get Your API Key</h2>
        <p className="text-muted-foreground">
          Connect your applications to AllSource with a secure API key.
        </p>
      </div>

      <Card className="mb-6">
        <CardContent className="p-6">
          {!apiKey ? (
            // Create form
            <div className="space-y-4">
              <div>
                <Label htmlFor="keyName">API Key Name</Label>
                <Input
                  id="keyName"
                  value={keyName}
                  onChange={(e) => setKeyName(e.target.value)}
                  placeholder="Enter a name for your API key"
                  className="mt-1.5"
                />
                <p className="mt-1 text-xs text-muted-foreground">
                  Use a descriptive name to identify this key later
                </p>
              </div>

              <div>
                <Label>Permissions</Label>
                <div className="mt-1.5 grid grid-cols-2 gap-2">
                  {["events:read", "events:write", "queries:execute", "projections:read"].map(
                    (scope) => (
                      <div
                        key={scope}
                        className="flex items-center gap-2 rounded-lg border border-border bg-muted/50 px-3 py-2"
                      >
                        <Check className="h-4 w-4 text-green-500" />
                        <span className="text-sm">{scope}</span>
                      </div>
                    )
                  )}
                </div>
              </div>

              {error && (
                <div className="rounded-lg border border-destructive/50 bg-destructive/10 p-3">
                  <p className="text-sm text-destructive">{error}</p>
                </div>
              )}

              <Button
                className="w-full"
                onClick={handleCreate}
                disabled={isCreating || !keyName.trim()}
              >
                {isCreating ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    Creating...
                  </>
                ) : (
                  <>
                    <Key className="mr-2 h-4 w-4" />
                    Generate API Key
                  </>
                )}
              </Button>
            </div>
          ) : (
            // Show key
            <div className="space-y-4">
              <div className="rounded-lg border border-yellow-500/50 bg-yellow-500/10 p-4">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="h-5 w-5 shrink-0 text-yellow-600" />
                  <div>
                    <p className="font-medium text-yellow-600">Save your API key now</p>
                    <p className="text-sm text-yellow-600/80">
                      You won't be able to see this key again after leaving this page. Copy it and
                      store it securely.
                    </p>
                  </div>
                </div>
              </div>

              <div>
                <Label>Your API Key</Label>
                <div className="mt-1.5 flex gap-2">
                  <div className="relative flex-1">
                    <Input
                      value={showKey ? apiKey : "•".repeat(apiKey.length)}
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

              <ApiKeyUsage apiKey={apiKey} />
            </div>
          )}
        </CardContent>
      </Card>

      {/* Continue button */}
      <div className="flex justify-center">
        <Button size="lg" onClick={onNext} disabled={!apiKey}>
          {apiKey ? (
            <>
              Continue
              <ArrowRight className="ml-2 h-4 w-4" />
            </>
          ) : (
            "Generate key to continue"
          )}
        </Button>
      </div>
    </div>
  );
}
