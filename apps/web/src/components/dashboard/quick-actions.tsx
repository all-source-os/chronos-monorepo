"use client";

import { Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import { BookOpen, FileText, Key, ListChecks, Plus, Terminal } from "lucide-react";
import Link from "next/link";

const actions = [
  {
    name: "Create Event",
    description: "Send a new event to the store",
    icon: Plus,
    href: "/dashboard/events?action=create",
    color: "bg-blue-500/10 text-blue-600 dark:text-blue-400",
  },
  {
    name: "Generate API Key",
    description: "Create credentials for your app",
    icon: Key,
    href: "/dashboard/api-keys?action=create",
    color: "bg-purple-500/10 text-purple-600 dark:text-purple-400",
  },
  {
    name: "Guided Setup",
    description: "Ingest and query a real event",
    icon: ListChecks,
    href: "/onboarding",
    color: "bg-cyan-500/10 text-cyan-600 dark:text-cyan-400",
  },
  {
    name: "Documentation",
    description: "Integrate AllSource",
    icon: BookOpen,
    href: "/docs",
    color: "bg-green-500/10 text-green-600 dark:text-green-400",
  },
  {
    name: "API Reference",
    description: "Explore available endpoints",
    icon: Terminal,
    href: "/docs/api",
    color: "bg-orange-500/10 text-orange-600 dark:text-orange-400",
  },
  {
    name: "Product Updates",
    description: "Read releases and engineering notes",
    icon: FileText,
    href: "/blog",
    color: "bg-pink-500/10 text-pink-600 dark:text-pink-400",
  },
];

export function QuickActions() {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="text-base font-medium">Quick Actions</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="grid gap-2 sm:grid-cols-2">
          {actions.map((action) => (
            <Link
              key={action.name}
              href={action.href}
              className="flex items-start gap-3 rounded-lg p-3 transition-colors hover:bg-muted"
            >
              <div className={`rounded-lg p-2 ${action.color}`}>
                <action.icon className="h-4 w-4" />
              </div>
              <div className="min-w-0">
                <p className="text-sm font-medium">{action.name}</p>
                <p className="truncate text-xs text-muted-foreground">{action.description}</p>
              </div>
            </Link>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
