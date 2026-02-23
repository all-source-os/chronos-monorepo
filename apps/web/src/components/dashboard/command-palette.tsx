"use client";

import { cn } from "@allsource/ui/utils";
import {
  Activity,
  CreditCard,
  ExternalLink,
  FileText,
  GitBranch,
  Key,
  LayoutDashboard,
  Plus,
  Search,
  Settings,
  Zap,
} from "lucide-react";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useState } from "react";

interface CommandPaletteProps {
  open: boolean;
  onClose: () => void;
}

interface Command {
  id: string;
  name: string;
  icon: React.ElementType;
  action: () => void;
  shortcut?: string;
  group: string;
}

export function CommandPalette({ open, onClose }: CommandPaletteProps) {
  const router = useRouter();
  const [search, setSearch] = useState("");
  const [selectedIndex, setSelectedIndex] = useState(0);

  const commands: Command[] = [
    // Navigation
    {
      id: "dashboard",
      name: "Go to Overview",
      icon: LayoutDashboard,
      action: () => router.push("/dashboard"),
      shortcut: "G O",
      group: "Navigation",
    },
    {
      id: "events",
      name: "Go to Events",
      icon: Activity,
      action: () => router.push("/dashboard/events"),
      shortcut: "G E",
      group: "Navigation",
    },
    {
      id: "pipelines",
      name: "Go to Pipelines",
      icon: GitBranch,
      action: () => router.push("/dashboard/pipelines"),
      shortcut: "G P",
      group: "Navigation",
    },
    {
      id: "api-keys",
      name: "Go to API Keys",
      icon: Key,
      action: () => router.push("/dashboard/api-keys"),
      shortcut: "G K",
      group: "Navigation",
    },
    {
      id: "billing",
      name: "Go to Billing",
      icon: CreditCard,
      action: () => router.push("/dashboard/billing"),
      shortcut: "G B",
      group: "Navigation",
    },
    {
      id: "settings",
      name: "Go to Settings",
      icon: Settings,
      action: () => router.push("/dashboard/settings"),
      shortcut: "G S",
      group: "Navigation",
    },
    {
      id: "demo",
      name: "Go to Demo Zone",
      icon: Zap,
      action: () => router.push("/demo"),
      shortcut: "G D",
      group: "Navigation",
    },
    // Actions
    {
      id: "create-event",
      name: "Create new event",
      icon: Plus,
      action: () => router.push("/dashboard/events?action=create"),
      group: "Actions",
    },
    {
      id: "create-key",
      name: "Create API key",
      icon: Key,
      action: () => router.push("/dashboard/api-keys?action=create"),
      group: "Actions",
    },
    // Resources
    {
      id: "docs",
      name: "Documentation",
      icon: FileText,
      action: () => window.open("https://docs.all-source.xyz", "_blank"),
      group: "Resources",
    },
    {
      id: "api-reference",
      name: "API Reference",
      icon: ExternalLink,
      action: () => window.open("https://docs.all-source.xyz/api", "_blank"),
      group: "Resources",
    },
  ];

  const filteredCommands = commands.filter((command) =>
    command.name.toLowerCase().includes(search.toLowerCase())
  );

  const groupedCommands = filteredCommands.reduce(
    (acc, command) => {
      if (!acc[command.group]) {
        acc[command.group] = [];
      }
      acc[command.group]!.push(command);
      return acc;
    },
    {} as Record<string, Command[]>
  );

  const flatFilteredCommands = Object.values(groupedCommands).flat();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (!open) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          setSelectedIndex((i) => (i < flatFilteredCommands.length - 1 ? i + 1 : 0));
          break;
        case "ArrowUp":
          e.preventDefault();
          setSelectedIndex((i) => (i > 0 ? i - 1 : flatFilteredCommands.length - 1));
          break;
        case "Enter":
          e.preventDefault();
          if (flatFilteredCommands[selectedIndex]) {
            flatFilteredCommands[selectedIndex].action();
            onClose();
          }
          break;
        case "Escape":
          e.preventDefault();
          onClose();
          break;
      }
    },
    [open, flatFilteredCommands, selectedIndex, onClose]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  useEffect(() => {
    setSelectedIndex(0);
  }, []);

  useEffect(() => {
    if (open) {
      setSearch("");
      setSelectedIndex(0);
    }
  }, [open]);

  if (!open) return null;

  return (
    <div className="fixed inset-0 z-50">
      {/* Backdrop */}
      <div className="absolute inset-0 bg-background/80 backdrop-blur-sm" onClick={onClose} />

      {/* Dialog */}
      <div className="relative mx-auto mt-[20vh] w-full max-w-lg px-4">
        <div className="overflow-hidden rounded-xl border border-border bg-background shadow-2xl">
          {/* Search input */}
          <div className="flex items-center gap-3 border-b border-border px-4 py-3">
            <Search className="h-5 w-5 text-muted-foreground" />
            <input
              type="text"
              placeholder="Search commands..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="flex-1 bg-transparent text-sm outline-none placeholder:text-muted-foreground"
            />
            <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border border-border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
              ESC
            </kbd>
          </div>

          {/* Results */}
          <div className="max-h-[300px] overflow-y-auto p-2">
            {flatFilteredCommands.length === 0 ? (
              <div className="px-4 py-8 text-center text-sm text-muted-foreground">
                No commands found
              </div>
            ) : (
              Object.entries(groupedCommands).map(([group, commands]) => (
                <div key={group} className="mb-2">
                  <div className="px-2 py-1 text-xs font-medium text-muted-foreground">{group}</div>
                  {commands.map((command) => {
                    const globalIndex = flatFilteredCommands.indexOf(command);
                    const isSelected = globalIndex === selectedIndex;

                    return (
                      <button
                        key={command.id}
                        onClick={() => {
                          command.action();
                          onClose();
                        }}
                        onMouseEnter={() => setSelectedIndex(globalIndex)}
                        className={cn(
                          "flex w-full items-center justify-between rounded-lg px-3 py-2 text-sm transition-colors",
                          isSelected ? "bg-muted" : "hover:bg-muted/50"
                        )}
                      >
                        <div className="flex items-center gap-3">
                          <command.icon className="h-4 w-4 text-muted-foreground" />
                          <span>{command.name}</span>
                        </div>
                        {command.shortcut && (
                          <kbd className="pointer-events-none inline-flex h-5 select-none items-center gap-1 rounded border border-border bg-muted px-1.5 font-mono text-[10px] font-medium text-muted-foreground">
                            {command.shortcut}
                          </kbd>
                        )}
                      </button>
                    );
                  })}
                </div>
              ))
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
