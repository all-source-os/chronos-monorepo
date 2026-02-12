"use client";

import { Button, Icons } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { BookOpen, ChevronRight, Code2, MessageSquare, PartyPopper, Sparkles } from "lucide-react";
import Link from "next/link";
import { useEffect, useState } from "react";

interface StepNextStepsProps {
  onComplete: () => void;
}

const resources = [
  {
    icon: BookOpen,
    title: "Documentation",
    description: "Learn everything about AllSource",
    href: "https://docs.allsource.dev",
    external: true,
  },
  {
    icon: Code2,
    title: "API Reference",
    description: "Explore all available endpoints",
    href: "https://docs.allsource.dev/api",
    external: true,
  },
  {
    icon: MessageSquare,
    title: "Join Discord",
    description: "Get help from the community",
    href: "https://discord.gg/allsource",
    external: true,
  },
  {
    icon: Sparkles,
    title: "MCP Integration",
    description: "Connect with Claude Desktop",
    href: "https://docs.allsource.dev/mcp",
    external: true,
  },
];

export function StepNextSteps({ onComplete }: StepNextStepsProps) {
  const [showConfetti, setShowConfetti] = useState(false);

  useEffect(() => {
    setShowConfetti(true);
    const timer = setTimeout(() => setShowConfetti(false), 3000);
    return () => clearTimeout(timer);
  }, []);

  return (
    <div className="mx-auto max-w-2xl text-center">
      {/* Confetti animation */}
      {showConfetti && (
        <div className="pointer-events-none fixed inset-0 z-50 overflow-hidden">
          {Array.from({ length: 50 }).map((_, i) => (
            <div
              key={i}
              className="absolute animate-confetti"
              style={{
                left: `${Math.random() * 100}%`,
                top: "-5%",
                animationDelay: `${Math.random() * 2}s`,
                animationDuration: `${3 + Math.random() * 2}s`,
              }}
            >
              <div
                className={cn(
                  "h-3 w-3 rotate-45",
                  [
                    "bg-primary",
                    "bg-blue-500",
                    "bg-green-500",
                    "bg-yellow-500",
                    "bg-pink-500",
                    "bg-purple-500",
                  ][Math.floor(Math.random() * 6)]
                )}
              />
            </div>
          ))}
        </div>
      )}

      {/* Success icon */}
      <div className="relative mx-auto mb-8 w-fit">
        <div className="absolute inset-0 animate-ping rounded-full bg-green-500/20" />
        <div className="relative flex h-20 w-20 items-center justify-center rounded-full bg-green-500/10">
          <PartyPopper className="h-10 w-10 text-green-500" />
        </div>
      </div>

      {/* Success message */}
      <h2 className="mb-2 text-3xl font-bold">You're All Set!</h2>
      <p className="mb-8 text-lg text-muted-foreground">
        Congratulations! You've completed the onboarding. You're ready to build amazing things with
        AllSource.
      </p>

      {/* Resources grid */}
      <div className="mb-8 grid gap-4 sm:grid-cols-2">
        {resources.map((resource) => {
          const Component = resource.external ? "a" : Link;
          const props = resource.external
            ? {
                href: resource.href,
                target: "_blank",
                rel: "noopener noreferrer",
              }
            : { href: resource.href };

          return (
            <Component
              key={resource.title}
              {...props}
              className="flex items-center gap-4 rounded-xl border border-border p-4 text-left transition-all hover:border-primary/50 hover:bg-muted/50"
            >
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-lg bg-primary/10">
                <resource.icon className="h-5 w-5 text-primary" />
              </div>
              <div className="flex-1">
                <p className="font-medium">{resource.title}</p>
                <p className="text-sm text-muted-foreground">{resource.description}</p>
              </div>
              <ChevronRight className="h-5 w-5 text-muted-foreground" />
            </Component>
          );
        })}
      </div>

      {/* CTA */}
      <Button size="lg" onClick={onComplete} className="px-8">
        Go to Dashboard
        <ChevronRight className="ml-2 h-4 w-4" />
      </Button>

      {/* Stats reminder */}
      <div className="mt-8 rounded-xl border border-border bg-muted/30 p-6">
        <div className="flex items-center justify-center gap-2 text-muted-foreground">
          <Icons.logo className="h-5 w-5 text-primary" />
          <span>AllSource Event Store</span>
        </div>
        <div className="mt-4 grid grid-cols-4 gap-4">
          <div>
            <p className="text-xl font-bold text-primary">469K</p>
            <p className="text-xs text-muted-foreground">events/sec</p>
          </div>
          <div>
            <p className="text-xl font-bold text-primary">11.9μs</p>
            <p className="text-xs text-muted-foreground">p99 latency</p>
          </div>
          <div>
            <p className="text-xl font-bold text-primary">27</p>
            <p className="text-xs text-muted-foreground">MCP tools</p>
          </div>
          <div>
            <p className="text-xl font-bold text-primary">~129MB</p>
            <p className="text-xs text-muted-foreground">footprint</p>
          </div>
        </div>
      </div>
    </div>
  );
}
