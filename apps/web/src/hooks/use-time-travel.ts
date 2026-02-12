"use client";

// Re-export everything from the context
export {
  type TimeTravelContextValue,
  TimeTravelProvider,
  type TimeTravelProviderProps,
  useTimeTravel,
  useTimeTravelOptional,
} from "@/lib/context/time-travel-context";

// Time travel presets for quick selection
export interface TimeTravelPreset {
  label: string;
  getDate: () => Date;
}

export const TIME_TRAVEL_PRESETS: TimeTravelPreset[] = [
  {
    label: "1 hour ago",
    getDate: () => new Date(Date.now() - 60 * 60 * 1000),
  },
  {
    label: "3 hours ago",
    getDate: () => new Date(Date.now() - 3 * 60 * 60 * 1000),
  },
  {
    label: "Yesterday 3am",
    getDate: () => {
      const date = new Date();
      date.setDate(date.getDate() - 1);
      date.setHours(3, 0, 0, 0);
      return date;
    },
  },
  {
    label: "Yesterday midnight",
    getDate: () => {
      const date = new Date();
      date.setDate(date.getDate() - 1);
      date.setHours(0, 0, 0, 0);
      return date;
    },
  },
  {
    label: "Last week",
    getDate: () => new Date(Date.now() - 7 * 24 * 60 * 60 * 1000),
  },
  {
    label: "2 weeks ago",
    getDate: () => new Date(Date.now() - 14 * 24 * 60 * 60 * 1000),
  },
];

// Demo-specific presets (for the Feb 11 incident scenario)
export const DEMO_PRESETS: TimeTravelPreset[] = [
  {
    label: "Before incident (Feb 10, 11pm)",
    getDate: () => new Date("2026-02-10T23:00:00Z"),
  },
  {
    label: "During incident (Feb 11, 3am)",
    getDate: () => new Date("2026-02-11T03:00:00Z"),
  },
  {
    label: "Peak incident (Feb 11, 4am)",
    getDate: () => new Date("2026-02-11T04:00:00Z"),
  },
  {
    label: "After resolution (Feb 11, 6am)",
    getDate: () => new Date("2026-02-11T06:00:00Z"),
  },
];

// Keyboard shortcut for time travel (Cmd/Ctrl + T)
export const TIME_TRAVEL_SHORTCUT = {
  key: "t",
  metaKey: true,
  description: "Open time travel picker",
};
