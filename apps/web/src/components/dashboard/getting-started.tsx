import { Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Activity, Check, Key, Search } from "lucide-react";
import Link from "next/link";

interface GettingStartedProps {
  hasApiKey: boolean;
  hasEvents: boolean;
}

export function GettingStarted({ hasApiKey, hasEvents }: GettingStartedProps) {
  const steps = [
    {
      title: "Create an API key",
      description: "Authenticate one application.",
      href: "/dashboard/api-keys?action=create",
      complete: hasApiKey,
      icon: Key,
    },
    {
      title: "Store an event",
      description: "Write real tenant data.",
      href: "/dashboard/events?action=create",
      complete: hasEvents,
      icon: Activity,
    },
    {
      title: "Inspect the stream",
      description: "Verify payload and history.",
      href: "/dashboard/events",
      complete: hasEvents,
      icon: Search,
    },
  ];
  const completed = steps.filter((step) => step.complete).length;

  return (
    <Card className="overflow-hidden border-primary/30">
      <CardContent className="p-0">
        <div className="flex flex-col gap-4 border-b border-border bg-primary/5 p-5 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <p className="text-xs font-semibold uppercase tracking-[0.16em] text-primary">
              Tenant setup · {completed}/{steps.length}
            </p>
            <h2 className="mt-1 text-lg font-semibold">Verify your first event path</h2>
            <p className="mt-1 text-sm text-muted-foreground">
              Create credentials, write tenant data, then inspect what AllSource stored.
            </p>
          </div>
          <Button asChild size="sm">
            <Link href="/onboarding">Open guided setup</Link>
          </Button>
        </div>
        <div className="grid md:grid-cols-3">
          {steps.map((step, index) => (
            <Link
              key={step.title}
              href={step.href}
              className={cn(
                "group flex items-start gap-3 p-5 transition-colors hover:bg-muted/60 focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring",
                index > 0 && "border-t border-border md:border-l md:border-t-0"
              )}
            >
              <span
                className={cn(
                  "flex h-9 w-9 shrink-0 items-center justify-center rounded-lg border",
                  step.complete
                    ? "border-green-500/30 bg-green-500/10 text-green-500"
                    : "border-border bg-background text-muted-foreground group-hover:text-primary"
                )}
              >
                {step.complete ? <Check className="h-4 w-4" /> : <step.icon className="h-4 w-4" />}
              </span>
              <span>
                <span className="block text-sm font-medium">{step.title}</span>
                <span className="mt-0.5 block text-xs text-muted-foreground">
                  {step.complete ? "Complete" : step.description}
                </span>
              </span>
            </Link>
          ))}
        </div>
      </CardContent>
    </Card>
  );
}
