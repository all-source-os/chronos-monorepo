"use client";

import { Button } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { AlertTriangle, History, RotateCcw } from "lucide-react";
import { useTimeTravelOptional } from "@/hooks/use-time-travel";

interface HistoricalModeBannerProps {
  className?: string;
}

export function HistoricalModeBanner({ className }: HistoricalModeBannerProps) {
  const { isHistorical, formatAsOf, returnToPresent } = useTimeTravelOptional();

  if (!isHistorical) {
    return null;
  }

  return (
    <div className={cn("border-b border-amber-500/30 bg-amber-500/10 px-4 py-2", className)}>
      <div className="container mx-auto flex max-w-7xl items-center justify-between gap-4">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-amber-500/20">
            <History className="h-4 w-4 text-amber-600 dark:text-amber-400" />
          </div>
          <div>
            <div className="flex items-center gap-2">
              <AlertTriangle className="h-3.5 w-3.5 text-amber-600 dark:text-amber-400" />
              <span className="text-sm font-medium text-amber-700 dark:text-amber-300">
                Time Travel Active
              </span>
            </div>
            <p className="text-xs text-amber-600/80 dark:text-amber-400/80">
              Viewing data from {formatAsOf("long")}
            </p>
          </div>
        </div>

        <Button
          size="sm"
          variant="outline"
          onClick={returnToPresent}
          className="gap-2 border-amber-500/50 bg-amber-500/10 text-amber-700 hover:bg-amber-500/20 dark:text-amber-300"
        >
          <RotateCcw className="h-3.5 w-3.5" />
          Return to Present
        </Button>
      </div>
    </div>
  );
}
