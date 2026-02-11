"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useId, useState, useEffect, Suspense } from "react";
import { Loader2 } from "lucide-react";
import { cn } from "@allsource/ui/utils";

import {
  BlurFade,
  Button,
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
  DotPattern,
  Icons,
} from "@allsource/ui";
import { getApiUrl } from "@/lib/api/client";

const ERROR_MESSAGES: Record<string, string> = {
  missing_token: "Authentication failed. Please try again.",
  invalid_token: "Session expired. Please sign in again.",
  auth_failed: "Authentication failed. Please try again.",
  access_denied: "Access was denied. Please try again.",
};

function LoginContent() {
  const searchParams = useSearchParams();
  const [loadingProvider, setLoadingProvider] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const errorId = useId();

  // Check for OAuth errors in URL
  useEffect(() => {
    const errorParam = searchParams.get("error");
    if (errorParam) {
      setError(ERROR_MESSAGES[errorParam] || "An error occurred. Please try again.");
    }
  }, [searchParams]);

  const handleOAuthLogin = (provider: "google" | "github") => {
    setLoadingProvider(provider);
    setError(null);
    // Redirect to OAuth provider via backend
    const apiUrl = getApiUrl();
    window.location.href = `${apiUrl}/api/auth/${provider}`;
  };

  const isDisabled = loadingProvider !== null;

  return (
    <div className="relative min-h-screen w-full overflow-hidden">
      {/* Background pattern */}
      <DotPattern
        className="opacity-50 dark:opacity-30 [mask-image:radial-gradient(ellipse_at_center,transparent_20%,black)]"
        cr={1}
        cx={1}
        cy={1}
      />

      {/* Content */}
      <div className="relative z-10 flex min-h-screen flex-col items-center justify-center px-4 sm:px-6">
        <BlurFade delay={0.1} inView>
          {/* Logo and branding */}
          <div className="mb-10 flex flex-col items-center gap-3">
            <div className="flex items-center gap-2.5">
              <Icons.logo className="h-10 w-10 text-primary" />
              <span className="text-3xl font-bold tracking-tight">AllSource</span>
            </div>
            <p className="text-muted-foreground">AI-native event store</p>
          </div>
        </BlurFade>

        <BlurFade delay={0.2} inView>
          <Card className="w-full max-w-[420px] border-border/50 bg-background/80 px-2 py-2 backdrop-blur-sm sm:px-4 sm:py-4">
            <CardHeader className="space-y-2 px-6 pb-0 pt-4 text-center sm:px-8 sm:pt-6">
              <CardTitle className="text-2xl font-semibold">Welcome back</CardTitle>
              <CardDescription className="text-base">
                Sign in to your account to continue
              </CardDescription>
            </CardHeader>
            <CardContent className="px-6 pb-6 pt-6 sm:px-8 sm:pb-8">
              {/* Error message */}
              {error && (
                <div
                  id={errorId}
                  role="alert"
                  className="mb-5 rounded-lg bg-destructive/10 px-4 py-3 text-sm text-destructive"
                >
                  {error}
                </div>
              )}

              {/* OAuth buttons */}
              <div className="grid gap-3">
                <Button
                  type="button"
                  variant="outline"
                  className="relative h-12 w-full bg-white hover:bg-gray-50 dark:bg-zinc-900 dark:hover:bg-zinc-800"
                  onClick={() => handleOAuthLogin("google")}
                  disabled={isDisabled}
                  aria-busy={loadingProvider === "google"}
                >
                  {loadingProvider === "google" ? (
                    <Loader2 className="h-5 w-5 animate-spin" aria-hidden="true" />
                  ) : (
                    <>
                      <Icons.google className="mr-2.5 h-5 w-5" aria-hidden="true" />
                      Continue with Google
                    </>
                  )}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  className="h-12 w-full bg-zinc-900 text-white hover:bg-zinc-800 dark:bg-white dark:text-zinc-900 dark:hover:bg-zinc-100"
                  onClick={() => handleOAuthLogin("github")}
                  disabled={isDisabled}
                  aria-busy={loadingProvider === "github"}
                >
                  {loadingProvider === "github" ? (
                    <Loader2 className="h-5 w-5 animate-spin" aria-hidden="true" />
                  ) : (
                    <>
                      <Icons.github className="mr-2.5 h-5 w-5" aria-hidden="true" />
                      Continue with GitHub
                    </>
                  )}
                </Button>
              </div>

              {/* Sign up link */}
              <p className="mt-6 text-center text-sm text-muted-foreground">
                Don&apos;t have an account?{" "}
                <Link
                  href="/signup"
                  className="font-medium text-primary underline-offset-4 transition-colors hover:underline focus:outline-none focus-visible:underline"
                >
                  Create one
                </Link>
              </p>
            </CardContent>
          </Card>
        </BlurFade>

        {/* Footer */}
        <BlurFade delay={0.3} inView>
          <p className="mt-10 text-center text-xs text-muted-foreground">
            By continuing, you agree to our{" "}
            <Link
              href="/terms"
              className="underline underline-offset-4 transition-colors hover:text-foreground focus:outline-none focus-visible:text-foreground"
            >
              Terms of Service
            </Link>{" "}
            and{" "}
            <Link
              href="/privacy"
              className="underline underline-offset-4 transition-colors hover:text-foreground focus:outline-none focus-visible:text-foreground"
            >
              Privacy Policy
            </Link>
          </p>
        </BlurFade>
      </div>
    </div>
  );
}

function LoginLoading() {
  return (
    <div className="relative min-h-screen w-full overflow-hidden">
      <DotPattern
        className="opacity-50 dark:opacity-30 [mask-image:radial-gradient(ellipse_at_center,transparent_20%,black)]"
        cr={1}
        cx={1}
        cy={1}
      />
      <div className="relative z-10 flex min-h-screen flex-col items-center justify-center px-4 sm:px-6">
        <div className="mb-10 flex flex-col items-center gap-3">
          <div className="flex items-center gap-2.5">
            <Icons.logo className="h-10 w-10 text-primary" />
            <span className="text-3xl font-bold tracking-tight">AllSource</span>
          </div>
          <p className="text-muted-foreground">AI-native event store</p>
        </div>
        <Card className="w-full max-w-[420px] border-border/50 bg-background/80 px-2 py-2 backdrop-blur-sm sm:px-4 sm:py-4">
          <CardHeader className="space-y-2 px-6 pb-0 pt-4 text-center sm:px-8 sm:pt-6">
            <CardTitle className="text-2xl font-semibold">Welcome back</CardTitle>
            <CardDescription className="text-base">
              Sign in to your account to continue
            </CardDescription>
          </CardHeader>
          <CardContent className="flex items-center justify-center px-6 pb-6 pt-6 sm:px-8 sm:pb-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default function LoginPage() {
  return (
    <Suspense fallback={<LoginLoading />}>
      <LoginContent />
    </Suspense>
  );
}
