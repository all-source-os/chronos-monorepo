"use client";

import Link from "next/link";
import { useId, useState } from "react";
import { Loader2 } from "lucide-react";

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
  Input,
  Label,
} from "@allsource/ui";

export default function LoginPage() {
  const [isLoading, setIsLoading] = useState(false);
  const [loadingProvider, setLoadingProvider] = useState<string | null>(null);
  const [email, setEmail] = useState("");
  const [password, setPassword] = useState("");
  const [error, setError] = useState<string | null>(null);

  // Unique IDs for accessibility
  const errorId = useId();
  const emailDescId = useId();

  const isDisabled = isLoading || loadingProvider !== null;

  const handleOAuthLogin = async (provider: "google" | "github") => {
    setLoadingProvider(provider);
    setError(null);
    // TODO: Redirect to OAuth provider
    // window.location.href = `${process.env.NEXT_PUBLIC_API_URL}/auth/${provider}`;

    // Simulate for demo
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setLoadingProvider(null);
    setError(`OAuth with ${provider} is not configured yet`);
  };

  const handleEmailLogin = async (e: React.FormEvent) => {
    e.preventDefault();
    setIsLoading(true);
    setError(null);

    // Basic validation
    if (!email || !password) {
      setError("Please enter your email and password");
      setIsLoading(false);
      return;
    }

    // TODO: Implement email login
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setIsLoading(false);
    setError("Email login is not configured yet");
  };

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
              <span className="text-3xl font-bold tracking-tight">Chronos</span>
            </div>
            <p className="text-muted-foreground">Event sourcing made simple</p>
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

              {/* OAuth buttons - Primary actions */}
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

              {/* Divider */}
              <div className="relative my-6">
                <div className="absolute inset-0 flex items-center">
                  <span className="w-full border-t border-border" />
                </div>
                <div className="relative flex justify-center text-xs uppercase">
                  <span className="bg-background px-3 text-muted-foreground">
                    or
                  </span>
                </div>
              </div>

              {/* Email form */}
              <form onSubmit={handleEmailLogin} noValidate>
                <div className="space-y-5">
                  {/* Email field */}
                  <div className="space-y-1.5">
                    <Label htmlFor="email" className="text-sm font-medium">
                      Email address
                    </Label>
                    <Input
                      id="email"
                      name="email"
                      type="email"
                      inputMode="email"
                      value={email}
                      onChange={(e) => setEmail(e.target.value)}
                      disabled={isDisabled}
                      required
                      autoComplete="email"
                      autoCapitalize="none"
                      autoCorrect="off"
                      spellCheck="false"
                      aria-invalid={error ? "true" : undefined}
                      aria-describedby={error ? errorId : emailDescId}
                      className="h-12 text-base"
                    />
                    <p id={emailDescId} className="sr-only">
                      Enter the email address associated with your account
                    </p>
                  </div>

                  {/* Password field */}
                  <div className="space-y-1.5">
                    <Label htmlFor="password" className="text-sm font-medium">
                      Password
                    </Label>
                    <Input
                      id="password"
                      name="password"
                      type="password"
                      value={password}
                      onChange={(e) => setPassword(e.target.value)}
                      disabled={isDisabled}
                      required
                      autoComplete="current-password"
                      aria-invalid={error ? "true" : undefined}
                      aria-describedby={error ? errorId : undefined}
                      className="h-12 text-base"
                    />
                  </div>
                </div>

                {/* Submit button */}
                <Button
                  type="submit"
                  className="mt-6 h-12 w-full text-base font-medium"
                  disabled={isDisabled}
                  aria-busy={isLoading}
                >
                  {isLoading ? (
                    <Loader2 className="h-5 w-5 animate-spin" aria-hidden="true" />
                  ) : (
                    "Sign in"
                  )}
                </Button>
              </form>

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
