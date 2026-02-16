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
import { Bell, Building2, Check, Loader2, Shield, User } from "lucide-react";
import { useState } from "react";
import { useAuthStore } from "@/lib/stores/auth-store";

type Tab = "profile" | "workspace" | "security" | "notifications";

export default function SettingsPage() {
  const { user, tenant } = useAuthStore();
  const [activeTab, setActiveTab] = useState<Tab>("profile");
  const [isSaving, setIsSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  // Form states
  const [profileData, setProfileData] = useState({
    name: user?.name || "",
    email: user?.email || "",
  });

  const [workspaceData, setWorkspaceData] = useState({
    name: tenant?.name || "",
    slug: tenant?.slug || "",
  });

  const handleSave = async () => {
    setIsSaving(true);
    await new Promise((resolve) => setTimeout(resolve, 1000));
    setIsSaving(false);
    setSaved(true);
    setTimeout(() => setSaved(false), 2000);
  };

  const tabs = [
    { id: "profile" as const, label: "Profile", icon: User },
    { id: "workspace" as const, label: "Workspace", icon: Building2 },
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
              <>
                <Card>
                  <CardHeader>
                    <CardTitle>Profile Information</CardTitle>
                    <CardDescription>Update your personal information</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    {/* Avatar */}
                    <div className="flex items-center gap-4">
                      {user?.avatar_url ? (
                        <img
                          src={user.avatar_url}
                          alt={user.name}
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
                          Managed by {user?.provider || "OAuth provider"}
                        </p>
                      </div>
                    </div>

                    {/* Name */}
                    <div>
                      <Label htmlFor="name">Full Name</Label>
                      <Input
                        id="name"
                        value={profileData.name}
                        onChange={(e) => setProfileData((p) => ({ ...p, name: e.target.value }))}
                        className="mt-1.5"
                      />
                    </div>

                    {/* Email */}
                    <div>
                      <Label htmlFor="email">Email Address</Label>
                      <Input
                        id="email"
                        type="email"
                        value={profileData.email}
                        disabled
                        className="mt-1.5"
                      />
                      <p className="mt-1 text-xs text-muted-foreground">
                        Email is managed by your authentication provider
                      </p>
                    </div>
                  </CardContent>
                </Card>

                {/* Save button */}
                <div className="flex justify-end">
                  <Button onClick={handleSave} disabled={isSaving}>
                    {saved ? (
                      <>
                        <Check className="mr-2 h-4 w-4" />
                        Saved
                      </>
                    ) : isSaving ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      "Save Changes"
                    )}
                  </Button>
                </div>
              </>
            )}

            {/* Workspace Tab */}
            {activeTab === "workspace" && (
              <>
                <Card>
                  <CardHeader>
                    <CardTitle>Workspace Settings</CardTitle>
                    <CardDescription>Manage your workspace details</CardDescription>
                  </CardHeader>
                  <CardContent className="space-y-4">
                    {/* Workspace icon */}
                    <div className="flex items-center gap-4">
                      <div className="flex h-16 w-16 items-center justify-center rounded-lg bg-primary/10 text-xl font-bold text-primary">
                        {tenant?.name?.charAt(0).toUpperCase() || "W"}
                      </div>
                      <div>
                        <p className="font-medium">{tenant?.name}</p>
                        <Badge variant="secondary" className="mt-1 capitalize">
                          {tenant?.subscription_tier || "free"} plan
                        </Badge>
                      </div>
                    </div>

                    {/* Name */}
                    <div>
                      <Label htmlFor="workspace-name">Workspace Name</Label>
                      <Input
                        id="workspace-name"
                        value={workspaceData.name}
                        onChange={(e) => setWorkspaceData((w) => ({ ...w, name: e.target.value }))}
                        className="mt-1.5"
                      />
                    </div>

                    {/* Slug */}
                    <div>
                      <Label htmlFor="workspace-slug">Workspace URL</Label>
                      <div className="mt-1.5 flex">
                        <span className="flex items-center rounded-l-md border border-r-0 border-input bg-muted px-3 text-sm text-muted-foreground">
                          all-source.xyz/
                        </span>
                        <Input
                          id="workspace-slug"
                          value={workspaceData.slug}
                          onChange={(e) =>
                            setWorkspaceData((w) => ({
                              ...w,
                              slug: e.target.value.toLowerCase().replace(/[^a-z0-9-]/g, ""),
                            }))
                          }
                          className="rounded-l-none"
                        />
                      </div>
                    </div>

                    {/* Tenant ID */}
                    <div>
                      <Label>Tenant ID</Label>
                      <div className="mt-1.5 flex items-center gap-2">
                        <code className="flex-1 rounded-md bg-muted px-3 py-2 font-mono text-sm">
                          {tenant?.id || "tenant-id"}
                        </code>
                      </div>
                      <p className="mt-1 text-xs text-muted-foreground">
                        Use this ID for API authentication
                      </p>
                    </div>
                  </CardContent>
                </Card>

                <div className="flex justify-end">
                  <Button onClick={handleSave} disabled={isSaving}>
                    {saved ? (
                      <>
                        <Check className="mr-2 h-4 w-4" />
                        Saved
                      </>
                    ) : isSaving ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        Saving...
                      </>
                    ) : (
                      "Save Changes"
                    )}
                  </Button>
                </div>
              </>
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
                            {user?.provider === "google" ? user.email : "Not connected"}
                          </p>
                        </div>
                      </div>
                      <Badge variant={user?.provider === "google" ? "default" : "outline"}>
                        {user?.provider === "google" ? "Connected" : "Connect"}
                      </Badge>
                    </div>

                    {/* GitHub */}
                    <div className="flex items-center justify-between rounded-lg border border-border p-4">
                      <div className="flex items-center gap-3">
                        <Icons.github className="h-6 w-6" />
                        <div>
                          <p className="font-medium">GitHub</p>
                          <p className="text-sm text-muted-foreground">
                            {user?.provider === "github" ? user.email : "Not connected"}
                          </p>
                        </div>
                      </div>
                      <Badge variant={user?.provider === "github" ? "default" : "outline"}>
                        {user?.provider === "github" ? "Connected" : "Connect"}
                      </Badge>
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
                  {[
                    {
                      title: "Usage Alerts",
                      description: "Get notified when approaching quota limits",
                      enabled: true,
                    },
                    {
                      title: "Pipeline Errors",
                      description: "Receive alerts when pipelines fail",
                      enabled: true,
                    },
                    {
                      title: "Security Alerts",
                      description: "Get notified about security-related events",
                      enabled: true,
                    },
                    {
                      title: "Product Updates",
                      description: "Learn about new features and improvements",
                      enabled: false,
                    },
                    {
                      title: "Tips & Tutorials",
                      description: "Receive helpful tips to get the most out of AllSource",
                      enabled: false,
                    },
                  ].map((pref) => (
                    <div key={pref.title} className="flex items-center justify-between">
                      <div>
                        <p className="font-medium">{pref.title}</p>
                        <p className="text-sm text-muted-foreground">{pref.description}</p>
                      </div>
                      <label className="relative inline-flex cursor-pointer items-center">
                        <input
                          type="checkbox"
                          defaultChecked={pref.enabled}
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
