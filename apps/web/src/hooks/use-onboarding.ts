"use client";

import { create } from "zustand";
import { persist } from "zustand/middleware";

interface OnboardingState {
  currentStep: number;
  completedSteps: number[];
  skipped: boolean;
  eventCreated: boolean;
  apiKeyCreated: boolean;

  // Actions
  setCurrentStep: (step: number) => void;
  completeStep: (step: number) => void;
  setSkipped: (skipped: boolean) => void;
  setEventCreated: (created: boolean) => void;
  setApiKeyCreated: (created: boolean) => void;
  reset: () => void;
  isStepCompleted: (step: number) => boolean;
}

const initialState = {
  currentStep: 0,
  completedSteps: [],
  skipped: false,
  eventCreated: false,
  apiKeyCreated: false,
};

export const useOnboarding = create<OnboardingState>()(
  persist(
    (set, get) => ({
      ...initialState,

      setCurrentStep: (step) => set({ currentStep: step }),

      completeStep: (step) =>
        set((state) => ({
          completedSteps: state.completedSteps.includes(step)
            ? state.completedSteps
            : [...state.completedSteps, step],
        })),

      setSkipped: (skipped) => set({ skipped }),

      setEventCreated: (created) => set({ eventCreated: created }),

      setApiKeyCreated: (created) => set({ apiKeyCreated: created }),

      reset: () => set(initialState),

      isStepCompleted: (step) => get().completedSteps.includes(step),
    }),
    {
      name: "onboarding-storage",
    }
  )
);

export const ONBOARDING_STEPS = [
  {
    id: "welcome",
    title: "Welcome to AllSource",
    description: "Let's get you set up",
  },
  {
    id: "create-event",
    title: "Create Your First Event",
    description: "See event sourcing in action",
  },
  {
    id: "explore",
    title: "Explore Time Travel",
    description: "Navigate through your event history",
  },
  {
    id: "api-key",
    title: "Get Your API Key",
    description: "Connect your applications",
  },
  {
    id: "next-steps",
    title: "You're All Set!",
    description: "What's next",
  },
];
