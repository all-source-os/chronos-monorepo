"use client";

import { Button, Icons } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Clock, Shield, Sparkles, Zap } from "lucide-react";
import { useAuthStore } from "@/lib/stores/auth-store";

interface StepWelcomeProps {
  onNext: () => void;
}

const features = [
  {
    icon: Zap,
    title: "469K events/sec",
    description: "High-performance ingestion",
  },
  {
    icon: Clock,
    title: "11.9μs latency",
    description: "Sub-microsecond queries",
  },
  {
    icon: Shield,
    title: "Immutable history",
    description: "Complete audit trails",
  },
  {
    icon: Sparkles,
    title: "27 MCP tools",
    description: "AI-native integration",
  },
];

export function StepWelcome({ onNext }: StepWelcomeProps) {
  const { user } = useAuthStore();

  return (
    <div className="flex flex-col items-center text-center">
      {/* Animated logo */}
      <div className="relative mb-8">
        <div className="absolute inset-0 animate-pulse rounded-full bg-primary/20 blur-xl" />
        <Icons.logo className="relative h-24 w-24 text-primary" />
      </div>

      {/* Welcome text */}
      <h1 className="mb-2 text-3xl font-bold tracking-tight md:text-4xl">
        Welcome to AllSource, {user?.name?.split(" ")[0] || "there"}!
      </h1>
      <p className="mb-8 max-w-md text-lg text-muted-foreground">
        You're about to unlock the power of AI-native event sourcing. Let's get you started.
      </p>

      {/* Features grid */}
      <div className="mb-8 grid w-full max-w-lg grid-cols-2 gap-4">
        {features.map((feature, index) => (
          <div
            key={feature.title}
            className={cn(
              "rounded-xl border border-border bg-card p-4 text-left transition-all",
              "animate-in fade-in-50 slide-in-from-bottom-4"
            )}
            style={{ animationDelay: `${index * 100}ms` }}
          >
            <feature.icon className="mb-2 h-5 w-5 text-primary" />
            <p className="font-semibold">{feature.title}</p>
            <p className="text-sm text-muted-foreground">{feature.description}</p>
          </div>
        ))}
      </div>

      {/* CTA */}
      <Button size="lg" onClick={onNext} className="px-8">
        Let's go
        <Sparkles className="ml-2 h-4 w-4" />
      </Button>
    </div>
  );
}
