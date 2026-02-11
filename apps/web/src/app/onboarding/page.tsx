"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { Button, Icons } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { ArrowLeft, Check, SkipForward } from "lucide-react";
import { useOnboarding, ONBOARDING_STEPS } from "@/hooks/use-onboarding";
import { useAuthStore } from "@/lib/stores/auth-store";
import { StepWelcome } from "@/components/onboarding/step-welcome";
import { StepCreateEvent } from "@/components/onboarding/step-create-event";
import { StepExplore } from "@/components/onboarding/step-explore";
import { StepApiKey } from "@/components/onboarding/step-api-key";
import { StepNextSteps } from "@/components/onboarding/step-next-steps";

export default function OnboardingPage() {
  const router = useRouter();
  const { login, setLoading } = useAuthStore();
  const {
    currentStep,
    setCurrentStep,
    completeStep,
    setSkipped,
    isStepCompleted,
  } = useOnboarding();

  // Verify session on mount
  useEffect(() => {
    const fetchSession = async () => {
      try {
        const response = await fetch("/api/auth/session");
        if (!response.ok) {
          router.push("/login");
          return;
        }
        const data = await response.json();
        if (data.data?.user) {
          login(data.data.user, data.data.tenant);
        } else {
          router.push("/login");
        }
      } catch {
        router.push("/login");
      } finally {
        setLoading(false);
      }
    };

    fetchSession();
  }, [login, router, setLoading]);

  const handleNext = () => {
    completeStep(currentStep);
    if (currentStep < ONBOARDING_STEPS.length - 1) {
      setCurrentStep(currentStep + 1);
    }
  };

  const handleBack = () => {
    if (currentStep > 0) {
      setCurrentStep(currentStep - 1);
    }
  };

  const handleSkip = () => {
    setSkipped(true);
    router.push("/dashboard");
  };

  const handleComplete = () => {
    completeStep(currentStep);
    router.push("/dashboard");
  };

  const renderStep = () => {
    switch (currentStep) {
      case 0:
        return <StepWelcome onNext={handleNext} />;
      case 1:
        return <StepCreateEvent onNext={handleNext} />;
      case 2:
        return <StepExplore onNext={handleNext} />;
      case 3:
        return <StepApiKey onNext={handleNext} />;
      case 4:
        return <StepNextSteps onComplete={handleComplete} />;
      default:
        return <StepWelcome onNext={handleNext} />;
    }
  };

  return (
    <div className="min-h-screen bg-background">
      {/* Header */}
      <header className="fixed left-0 right-0 top-0 z-50 border-b border-border bg-background/95 backdrop-blur">
        <div className="mx-auto flex h-16 max-w-4xl items-center justify-between px-4">
          <div className="flex items-center gap-4">
            {currentStep > 0 && currentStep < ONBOARDING_STEPS.length - 1 && (
              <Button variant="ghost" size="icon" onClick={handleBack}>
                <ArrowLeft className="h-4 w-4" />
              </Button>
            )}
            <div className="flex items-center gap-2">
              <Icons.logo className="h-6 w-6 text-primary" />
              <span className="font-semibold">AllSource</span>
            </div>
          </div>

          {/* Progress indicator */}
          <div className="hidden items-center gap-2 sm:flex">
            {ONBOARDING_STEPS.map((step, index) => (
              <div key={step.id} className="flex items-center gap-2">
                <div
                  className={cn(
                    "flex h-8 w-8 items-center justify-center rounded-full text-sm font-medium transition-all",
                    index === currentStep
                      ? "bg-primary text-primary-foreground"
                      : index < currentStep || isStepCompleted(index)
                        ? "bg-green-500 text-white"
                        : "bg-muted text-muted-foreground"
                  )}
                >
                  {index < currentStep || isStepCompleted(index) ? (
                    <Check className="h-4 w-4" />
                  ) : (
                    index + 1
                  )}
                </div>
                {index < ONBOARDING_STEPS.length - 1 && (
                  <div
                    className={cn(
                      "h-0.5 w-8 transition-all",
                      index < currentStep ? "bg-green-500" : "bg-muted"
                    )}
                  />
                )}
              </div>
            ))}
          </div>

          {/* Mobile progress */}
          <div className="flex items-center gap-2 sm:hidden">
            <span className="text-sm text-muted-foreground">
              {currentStep + 1} / {ONBOARDING_STEPS.length}
            </span>
          </div>

          {/* Skip button */}
          {currentStep < ONBOARDING_STEPS.length - 1 && (
            <Button variant="ghost" size="sm" onClick={handleSkip}>
              <SkipForward className="mr-1.5 h-4 w-4" />
              Skip
            </Button>
          )}
        </div>
      </header>

      {/* Main content */}
      <main className="flex min-h-screen items-center justify-center px-4 pt-24 pb-12">
        <div className="w-full max-w-4xl">
          {/* Step content with animation */}
          <div
            key={currentStep}
            className="animate-in fade-in-50 slide-in-from-right-4 duration-300"
          >
            {renderStep()}
          </div>
        </div>
      </main>

      {/* Footer with step info */}
      <footer className="fixed bottom-0 left-0 right-0 border-t border-border bg-background py-4">
        <div className="mx-auto flex max-w-4xl items-center justify-center px-4">
          <div className="text-center">
            <p className="text-sm font-medium">
              {ONBOARDING_STEPS[currentStep]?.title}
            </p>
            <p className="text-xs text-muted-foreground">
              {ONBOARDING_STEPS[currentStep]?.description}
            </p>
          </div>
        </div>
      </footer>

      {/* Confetti animation styles */}
      <style jsx global>{`
        @keyframes confetti {
          0% {
            transform: translateY(0) rotate(0deg);
            opacity: 1;
          }
          100% {
            transform: translateY(100vh) rotate(720deg);
            opacity: 0;
          }
        }
        .animate-confetti {
          animation: confetti linear forwards;
        }
      `}</style>
    </div>
  );
}
