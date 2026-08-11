"use client";

import {
  Button,
  Calendar as CalendarComponent,
  Popover,
  PopoverContent,
  PopoverTrigger,
} from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { CalendarIcon, ChevronDown, Clock, History, RotateCcw, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import {
  DEMO_PRESETS,
  TIME_TRAVEL_PRESETS,
  TIME_TRAVEL_SHORTCUT,
  useTimeTravel,
} from "@/hooks/use-time-travel";

interface TimeTravelPickerProps {
  showDemoPresets?: boolean;
  className?: string;
}

/**
 * Local-calendar `YYYY-MM-DD`, replacing date-fns `format(date, "yyyy-MM-dd")`.
 *
 * date-fns entered the dashboard boot bundle for this single call (the header
 * renders this picker on every dashboard route). This is the whole of what was
 * used.
 *
 * NOT `toISOString().slice(0, 10)`: that converts to UTC first, so anyone east
 * of Greenwich after 00:00 local — or west of it before 00:00 — gets the wrong
 * calendar day, which on a time-travel control silently queries the wrong day.
 */
function toLocalYmd(date: Date): string {
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${date.getFullYear()}-${month}-${day}`;
}

export function TimeTravelPicker({ showDemoPresets = false, className }: TimeTravelPickerProps) {
  const { asOf, setAsOf, isHistorical, returnToPresent, formatAsOf } = useTimeTravel();
  const [isOpen, setIsOpen] = useState(false);
  const [customDate, setCustomDate] = useState("");
  const [customTime, setCustomTime] = useState("");

  // Initialize custom inputs when panel opens
  useEffect(() => {
    // Date and time must come from the SAME clock. These previously took the
    // date from `toISOString()` (UTC) and the time from `toTimeString()`
    // (local), so for any user not on UTC the two inputs disagreed for part of
    // every day — and `handleCustomDateTimeApply` recombines them into one
    // local `Date`, so the picker queried a point up to a day off.
    if (isOpen && asOf) {
      setCustomDate(toLocalYmd(asOf));
      setCustomTime(asOf.toTimeString().slice(0, 5));
    } else if (isOpen) {
      const now = new Date();
      setCustomDate(toLocalYmd(now));
      setCustomTime(now.toTimeString().slice(0, 5));
    }
  }, [isOpen, asOf]);

  // Keyboard shortcut handler
  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === TIME_TRAVEL_SHORTCUT.key) {
        e.preventDefault();
        setIsOpen((prev) => !prev);
      }
      if (e.key === "Escape" && isOpen) {
        setIsOpen(false);
      }
    },
    [isOpen]
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  const handleCustomDateTimeApply = () => {
    if (customDate && customTime) {
      const dateTime = new Date(`${customDate}T${customTime}:00`);
      if (!Number.isNaN(dateTime.getTime())) {
        setAsOf(dateTime);
        setIsOpen(false);
      }
    }
  };

  const handlePresetClick = (getDate: () => Date) => {
    setAsOf(getDate());
    setIsOpen(false);
  };

  const handleReturnToPresent = () => {
    returnToPresent();
    setIsOpen(false);
  };

  const presets = showDemoPresets ? DEMO_PRESETS : TIME_TRAVEL_PRESETS;

  return (
    <div className={cn("relative", className)}>
      {/* Trigger button */}
      <button
        onClick={() => setIsOpen(!isOpen)}
        className={cn(
          "flex items-center gap-2 rounded-lg border px-3 py-1.5 text-sm transition-colors",
          isHistorical
            ? "border-amber-500/50 bg-amber-500/10 text-amber-600 hover:bg-amber-500/20 dark:text-amber-400"
            : "border-border bg-muted/50 text-muted-foreground hover:bg-muted"
        )}
      >
        {isHistorical ? (
          <>
            <History className="h-4 w-4" />
            <span className="hidden sm:inline">{formatAsOf()}</span>
            <span className="sm:hidden">Historical</span>
          </>
        ) : (
          <>
            <Clock className="h-4 w-4" />
            <span className="hidden sm:inline">Time Travel</span>
          </>
        )}
        <ChevronDown className="h-3 w-3 opacity-50" />
      </button>

      {/* Popover */}
      {isOpen && (
        <>
          {/* Backdrop */}
          <div className="fixed inset-0 z-40" onClick={() => setIsOpen(false)} />

          {/* Panel */}
          <div className="absolute right-0 top-full z-50 mt-2 w-80 rounded-xl border border-border bg-background p-4 shadow-lg">
            {/* Header */}
            <div className="mb-4 flex items-center justify-between">
              <h3 className="flex items-center gap-2 text-sm font-medium">
                <History className="h-4 w-4" />
                Time Travel
              </h3>
              <div className="flex items-center gap-2">
                <kbd className="rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                  <span className="text-xs">&#8984;</span>T
                </kbd>
                <button onClick={() => setIsOpen(false)} className="rounded p-1 hover:bg-muted">
                  <X className="h-4 w-4" />
                </button>
              </div>
            </div>

            {/* Current state indicator */}
            {isHistorical && (
              <div className="mb-4 rounded-lg border border-amber-500/30 bg-amber-500/10 p-3">
                <div className="flex items-center justify-between">
                  <div>
                    <p className="text-xs font-medium text-amber-600 dark:text-amber-400">
                      Viewing historical data
                    </p>
                    <p className="text-sm font-medium">{formatAsOf("long")}</p>
                  </div>
                  <Button
                    size="sm"
                    variant="outline"
                    onClick={handleReturnToPresent}
                    className="gap-1 border-amber-500/50 text-amber-600 hover:bg-amber-500/20 dark:text-amber-400"
                  >
                    <RotateCcw className="h-3 w-3" />
                    Present
                  </Button>
                </div>
              </div>
            )}

            {/* Quick presets */}
            <div className="mb-4">
              <p className="mb-2 text-xs font-medium text-muted-foreground">Quick presets</p>
              <div className="grid grid-cols-2 gap-2">
                {presets.map((preset) => (
                  <button
                    key={preset.label}
                    onClick={() => handlePresetClick(preset.getDate)}
                    className="rounded-lg border border-border px-3 py-2 text-left text-xs transition-colors hover:bg-muted"
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>

            {/* Custom date/time */}
            <div>
              <p className="mb-2 text-xs font-medium text-muted-foreground">Custom date & time</p>
              <div className="flex gap-2">
                <div className="flex-1">
                  <Popover>
                    <PopoverTrigger asChild>
                      <Button
                        variant="outline"
                        className={cn(
                          "w-full justify-start text-left font-normal text-sm",
                          !customDate && "text-muted-foreground"
                        )}
                      >
                        <CalendarIcon className="mr-2 h-4 w-4" />
                        {customDate || "Pick date"}
                      </Button>
                    </PopoverTrigger>
                    <PopoverContent className="w-auto p-0" align="start">
                      <CalendarComponent
                        mode="single"
                        selected={customDate ? new Date(`${customDate}T00:00:00`) : undefined}
                        onSelect={(date) => {
                          if (date) {
                            setCustomDate(toLocalYmd(date));
                          }
                        }}
                        autoFocus
                      />
                    </PopoverContent>
                  </Popover>
                </div>
                <div className="w-24">
                  <label className="sr-only">Time</label>
                  <input
                    type="time"
                    value={customTime}
                    onChange={(e) => setCustomTime(e.target.value)}
                    className="w-full rounded-lg border border-border bg-muted/50 px-3 py-2 text-sm outline-none focus:border-primary"
                  />
                </div>
              </div>
              <Button
                size="sm"
                className="mt-2 w-full"
                onClick={handleCustomDateTimeApply}
                disabled={!customDate || !customTime}
              >
                <History className="mr-2 h-4 w-4" />
                Travel to this time
              </Button>
            </div>
          </div>
        </>
      )}
    </div>
  );
}
