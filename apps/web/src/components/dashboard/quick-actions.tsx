"use client";

import Link from "next/link";
import { Card, CardContent, CardHeader, CardTitle } from "@allsource/ui";
import {
  Plus,
  Key,
  FileText,
  MessageSquare,
  BookOpen,
  Terminal,
} from "lucide-react";

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
    name: "View Documentation",
    description: "Learn how to integrate AllSource",
    icon: BookOpen,
    href: "https://docs.allsource.dev",
    external: true,
    color: "bg-green-500/10 text-green-600 dark:text-green-400",
  },
  {
    name: "API Reference",
    description: "Explore available endpoints",
    icon: Terminal,
    href: "https://docs.allsource.dev/api",
    external: true,
    color: "bg-orange-500/10 text-orange-600 dark:text-orange-400",
  },
  {
    name: "Join Discord",
    description: "Get help from the community",
    icon: MessageSquare,
    href: "https://discord.gg/allsource",
    external: true,
    color: "bg-indigo-500/10 text-indigo-600 dark:text-indigo-400",
  },
  {
    name: "Changelog",
    description: "See what's new in AllSource",
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
          {actions.map((action) => {
            const Component = action.external ? "a" : Link;
            const props = action.external
              ? { href: action.href, target: "_blank", rel: "noopener noreferrer" }
              : { href: action.href };

            return (
              <Component
                key={action.name}
                {...props}
                className="flex items-start gap-3 rounded-lg p-3 transition-colors hover:bg-muted"
              >
                <div className={`rounded-lg p-2 ${action.color}`}>
                  <action.icon className="h-4 w-4" />
                </div>
                <div className="min-w-0">
                  <p className="text-sm font-medium">{action.name}</p>
                  <p className="truncate text-xs text-muted-foreground">
                    {action.description}
                  </p>
                </div>
              </Component>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
