"use client";

import {
  Badge,
  BlurFade,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  Icons,
  Input,
  Label,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Bell, Check, Copy, Shield, User } from "lucide-react";
import { useState } from "react";
import { useAuthStore } from "@/lib/stores/auth-store";
import {
  useNotificationPreferences,
  type NotificationPreferences,
} from "@/hooks/use-notification-preferences";

type Tab = "profile" | "security" | "notifications";

function parseProvider(user: { id: string; provider?: string } | null): string | null {
  if (!user) return null;
  // Prefer explicit provider field from JWT (Phase 5)
  if (user.provider && user.provider !== "email") return user.provider;
  // Fallback: parse from user ID format "oauth:{provider}:{providerID}"
  if (user.id?.startsWith("oauth:")) {
    const parts = user.id.split(":");
    if (parts.length >= 2 && parts[1]) return parts[1];
  }
  return user.provider ?? null;
}

export default function SettingsPage() {
  const { user, tenant, coreApiKey } = useAuthStore();
  const provider = parseProvider(user);
  const [apiKeyCopied, setApiKeyCopied] = useState(false);

  const handleCopyApiKey = async () => {
    if (!coreApiKey) return;
    await navigator.clipboard.writeText(coreApiKey);
    setApiKeyCopied(true);
    setTimeout(() => setApiKeyCopied(false), 2000);
  };
  const { preferences, updatePreference } = useNotificationPreferences();
  const [activeTab, setActiveTab] = useState<Tab>("profile");
  const tabs = [
    { id: "profile" as const, label: "Profile", icon: User },
    { id: "security" as const, label: "Security", icon: Shield },
    { id: "notifications" as const, label: "Notifications", icon: Bell },
  ];

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div>
          <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Settings</h1>
          <p className="mt-1 text-muted-foreground">
            Manage your account and workspace preferences
          </p>
        </div>
      </BlurFade>

      {/* Tabs and Content */}
      <BlurFade delay={0.2} inView>
        <div className="flex flex-col gap-6 lg:flex-row">
          {/* Sidebar tabs */}
          <nav className="flex gap-2 lg:w-48 lg:flex-col">
            {tabs.map((tab) => (
              <button
                key={tab.id}
                onClick={() => setActiveTab(tab.id)}
                className={cn(
                  "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                  activeTab === tab.id
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-muted hover:text-foreground"
                )}
              >
                <tab.icon className="h-4 w-4" />
                {tab.label}
              </button>
            ))}
          </nav>

          {/* Content */}
          <div className="flex-1 space-y-6">
            {/* Profile Tab */}
            {activeTab === "profile" && (
              <Card>
                <CardHeader>
                  <CardTitle>Profile Information</CardTitle>
                  <CardDescription>Your profile is managed by your authentication provider</CardDescription>
                </CardHeader>
                <CardContent className="space-y-4">
                  {/* Avatar */}
                  <div className="flex items-center gap-4">
                    {user?.avatar_url ? (
                      <img
                        src={user.avatar_url}
                        alt={user?.name || "Profile"}
                        className="h-16 w-16 rounded-full"
                      />
                    ) : (
                      <div className="flex h-16 w-16 items-center justify-center rounded-full bg-primary/10 text-xl font-medium text-primary">
                        {user?.name?.charAt(0).toUpperCase() || "U"}
                      </div>
                    )}
                    <div>
                      <p className="text-sm font-medium">Profile Picture</p>
                      <p className="text-xs text-muted-foreground">
                        Managed by {provider || "your OAuth provider"}
                      </p>
                    </div>
                  </div>

                  {/* Name */}
                  <div>
                    <Label>Full Name</Label>
                    <Input
                      value={user?.name || ""}
                      disabled
                      className="mt-1.5"
                    />
                  </div>

                  {/* Email */}
                  <div>
                    <Label>Email Address</Label>
                    <Input
                      type="email"
                      value={user?.email || ""}
                      disabled
                      className="mt-1.5"
                    />
                    <p className="mt-1 text-xs text-muted-foreground">
                      Email is managed by your authentication provider
                    </p>
                  </div>

                  {/* Tenant ID */}
                  <div>
                    <Label>Tenant ID</Label>
                    <div className="mt-1.5 flex items-center gap-2">
                      <code className="flex-1 rounded-md bg-muted px-3 py-2 font-mono text-sm">
                        {tenant?.id || "—"}
                      </code>
                    </div>
                    <p className="mt-1 text-xs text-muted-foreground">
                      Use this ID for API authentication
                    </p>
                  </div>

                  {/* Sync API Key */}
                  <div>
                    <Label>Sync API Key</Label>
                    {coreApiKey ? (
                      <>
                        <div className="mt-1.5 flex items-center gap-2">
                          <Input
                            value={coreApiKey}
                            readOnly
                            className="flex-1 font-mono text-sm"
                          />
                          <Button variant="outline" size="icon" onClick={handleCopyApiKey}>
                            {apiKeyCopied ? (
                              <Check className="h-4 w-4 text-green-500" />
                            ) : (
                              <Copy className="h-4 w-4" />
                            )}
                          </Button>
                        </div>
                        <p className="mt-1 text-xs text-muted-foreground">
                          Use this key in <code className="font-mono">.chronis/config.toml</code> for{" "}
                          <code className="font-mono">cn sync</code>. Keep it secret.
                        </p>
                        <pre className="mt-2 rounded-md bg-muted p-3 text-xs font-mono text-muted-foreground">
                          {`[remote]\nurl = "https://api.allsource.dev"\napi_key = "${coreApiKey}"`}
                        </pre>
                      </>
                    ) : (
                      <p className="mt-1.5 text-sm text-muted-foreground">
                        Log out and back in to generate your sync API key.
                      </p>
                    )}
                  </div>
                </CardContent>
              </Card>
            )}

            {/* Security Tab */}
            {activeTab === "security" && (
              <>
                <Card>
                  <CardHeader>
                    <CardTitle>Connected Accounts</CardTitle>
                    <CardDescription>Manage your authentication providers</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    {/* Google */}
                    <div className="flex items-center justify-between rounded-lg border border-border p-4">
                      <div className="flex items-center gap-3">
                        <Icons.google className="h-6 w-6" />
                        <div>
                          <p className="font-medium">Google</p>
                          <p className="text-sm text-muted-foreground">
                            {provider === "google" ? user?.email : "Not connected"}
                          </p>
                        </div>
                      </div>
                      {provider === "google" ? (
                        <Badge variant="default">Connected</Badge>
                      ) : (
                        <Badge variant="outline" className="opacity-50">Not available</Badge>
                      )}
                    </div>

                    {/* GitHub */}
                    <div className="flex items-center justify-between rounded-lg border border-border p-4">
                      <div className="flex items-center gap-3">
                        <Icons.github className="h-6 w-6" />
                        <div>
                          <p className="font-medium">GitHub</p>
                          <p className="text-sm text-muted-foreground">
                            {provider === "github" ? user?.email : "Not connected"}
                          </p>
                        </div>
                      </div>
                      {provider === "github" ? (
                        <Badge variant="default">Connected</Badge>
                      ) : (
                        <Badge variant="outline" className="opacity-50">Not available</Badge>
                      )}
                    </div>
                  </CardContent>
                </Card>

                <Card>
                  <CardHeader>
                    <CardTitle>Active Sessions</CardTitle>
                    <CardDescription>Devices where you're currently logged in</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <div className="rounded-lg border border-border p-4">
                      <div className="flex items-center justify-between">
                        <div>
                          <p className="font-medium">Current Session</p>
                          <p className="text-sm text-muted-foreground">Active now</p>
                        </div>
                        <Badge variant="secondary">This device</Badge>
                      </div>
                    </div>
                  </CardContent>
                </Card>

                <Card className="border-destructive/50">
                  <CardHeader>
                    <CardTitle className="text-destructive">Danger Zone</CardTitle>
                    <CardDescription>Irreversible and destructive actions</CardDescription>
                  </CardHeader>
                  <CardContent>
                    <Button variant="destructive">Delete Account</Button>
                  </CardContent>
                </Card>
              </>
            )}

            {/* Notifications Tab */}
            {activeTab === "notifications" && (
              <Card>
                <CardHeader>
                  <CardTitle>Notification Preferences</CardTitle>
                  <CardDescription>Choose what updates you want to receive</CardDescription>
                </CardHeader>
                <CardContent className="space-y-6">
                  {([
                    {
                      key: "usage_alerts" as const,
                      title: "Usage Alerts",
                      description: "Get notified when approaching quota limits",
                    },
                    {
                      key: "pipeline_errors" as const,
                      title: "Pipeline Errors",
                      description: "Receive alerts when pipelines fail",
                    },
                    {
                      key: "security_alerts" as const,
                      title: "Security Alerts",
                      description: "Get notified about security-related events",
                    },
                    {
                      key: "product_updates" as const,
                      title: "Product Updates",
                      description: "Learn about new features and improvements",
                    },
                    {
                      key: "tips" as const,
                      title: "Tips & Tutorials",
                      description: "Receive helpful tips to get the most out of AllSource",
                    },
                  ] satisfies { key: keyof NotificationPreferences; title: string; description: string }[]).map((pref) => (
                    <div key={pref.key} className="flex items-center justify-between">
                      <div>
                        <p className="font-medium">{pref.title}</p>
                        <p className="text-sm text-muted-foreground">{pref.description}</p>
                      </div>
                      <label className="relative inline-flex cursor-pointer items-center">
                        <input
                          type="checkbox"
                          checked={preferences[pref.key]}
                          onChange={(e) => updatePreference(pref.key, e.target.checked)}
                          className="peer sr-only"
                        />
                        <div className="h-6 w-11 rounded-full bg-muted peer-checked:bg-primary peer-focus:ring-2 peer-focus:ring-primary peer-focus:ring-offset-2 after:absolute after:left-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:bg-white after:transition-all peer-checked:after:translate-x-full" />
                      </label>
                    </div>
                  ))}
                </CardContent>
              </Card>
            )}
          </div>
        </div>
      </BlurFade>
    </div>
  );
}
