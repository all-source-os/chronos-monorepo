"use client";

import { createContext, type ReactNode, useCallback, useContext, useMemo, useState } from "react";

export interface TimeTravelContextValue {
  /** The timestamp to view data as-of, null means present */
  asOf: Date | null;
  /** Set the as-of timestamp for time travel */
  setAsOf: (date: Date | null) => void;
  /** Whether we're viewing historical data */
  isHistorical: boolean;
  /** Return to present (clear as-of timestamp) */
  returnToPresent: () => void;
  /** Get the as-of timestamp as ISO string for API calls */
  asOfIso: string | null;
  /** Format the as-of timestamp for display */
  formatAsOf: (format?: "short" | "long") => string;
}

const TimeTravelContext = createContext<TimeTravelContextValue | null>(null);

export interface TimeTravelProviderProps {
  children: ReactNode;
  /** Initial as-of timestamp */
  initialAsOf?: Date | null;
}

export function TimeTravelProvider({ children, initialAsOf = null }: TimeTravelProviderProps) {
  const [asOf, setAsOfState] = useState<Date | null>(initialAsOf);

  const setAsOf = useCallback((date: Date | null) => {
    setAsOfState(date);
  }, []);

  const returnToPresent = useCallback(() => {
    setAsOfState(null);
  }, []);

  const isHistorical = asOf !== null;

  const asOfIso = useMemo(() => {
    return asOf ? asOf.toISOString() : null;
  }, [asOf]);

  const formatAsOf = useCallback(
    (format: "short" | "long" = "short") => {
      if (!asOf) return "Present";

      const options: Intl.DateTimeFormatOptions =
        format === "short"
          ? {
              month: "short",
              day: "numeric",
              hour: "numeric",
              minute: "2-digit",
            }
          : {
              weekday: "short",
              month: "short",
              day: "numeric",
              year: "numeric",
              hour: "numeric",
              minute: "2-digit",
              second: "2-digit",
            };

      return asOf.toLocaleString(undefined, options);
    },
    [asOf]
  );

  const value = useMemo<TimeTravelContextValue>(
    () => ({
      asOf,
      setAsOf,
      isHistorical,
      returnToPresent,
      asOfIso,
      formatAsOf,
    }),
    [asOf, setAsOf, isHistorical, returnToPresent, asOfIso, formatAsOf]
  );

  return <TimeTravelContext.Provider value={value}>{children}</TimeTravelContext.Provider>;
}

export function useTimeTravel(): TimeTravelContextValue {
  const context = useContext(TimeTravelContext);

  if (!context) {
    throw new Error("useTimeTravel must be used within a TimeTravelProvider");
  }

  return context;
}

/** Hook that returns time travel context if available, or defaults for non-time-travel pages */
export function useTimeTravelOptional(): TimeTravelContextValue {
  const context = useContext(TimeTravelContext);

  // Return default values if not in a TimeTravelProvider
  if (!context) {
    return {
      asOf: null,
      setAsOf: () => {},
      isHistorical: false,
      returnToPresent: () => {},
      asOfIso: null,
      formatAsOf: () => "Present",
    };
  }

  return context;
}
