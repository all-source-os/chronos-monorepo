"use client";

import { BlurFade, Button, Card, CardContent, DotPattern, Icons } from "@allsource/ui";
import { CheckCircle2, Loader2, Mail, XCircle } from "lucide-react";
import Link from "next/link";
import { useRouter, useSearchParams } from "next/navigation";
import { Suspense, useEffect, useState } from "react";

type VerificationState = "verifying" | "success" | "error" | "expired";

function VerifyEmailContent() {
  const router = useRouter();
  const searchParams = useSearchParams();
  const [state, setState] = useState<VerificationState>("verifying");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const verifyEmail = async () => {
      const token = searchParams.get("token");

      if (!token) {
        setState("error");
        setError("Missing verification token.");
        return;
      }

      try {
        const response = await fetch("/api/auth/verify-email", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ token }),
        });

        const data = await response.json();

        if (!response.ok) {
          if (response.status === 410 || data.error?.code === "token_expired") {
            setState("expired");
          } else {
            setState("error");
            setError(data.error?.message || data.message || "Verification failed.");
          }
          return;
        }

        setState("success");

        // Auto-redirect to login after 3 seconds
        setTimeout(() => {
          router.push("/login?verified=true");
        }, 3000);
      } catch {
        setState("error");
        setError("Failed to verify email. Please try again.");
      }
    };

    verifyEmail();
  }, [searchParams, router]);

  const handleResendEmail = async () => {
    const email = searchParams.get("email");
    if (!email) {
      router.push("/signup");
      return;
    }

    try {
      await fetch("/api/auth/resend-verification", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ email }),
      });
      // Redirect to signup with success message
      router.push("/signup?resent=true");
    } catch {
      // Still redirect even on error
      router.push("/signup");
    }
  };

  return (
    <div className="relative min-h-screen w-full overflow-hidden">
      <DotPattern
        className="opacity-50 dark:opacity-30 [mask-image:radial-gradient(ellipse_at_center,transparent_20%,black)]"
        cr={1}
        cx={1}
        cy={1}
      />

      <div className="relative z-10 flex min-h-screen flex-col items-center justify-center px-4 sm:px-6">
        <BlurFade delay={0.1} inView>
          <div className="mb-10 flex flex-col items-center gap-3">
            <div className="flex items-center gap-2.5">
              <Icons.logo className="h-10 w-10 text-primary" />
              <span className="text-3xl font-bold tracking-tight">AllSource</span>
            </div>
          </div>
        </BlurFade>

        <BlurFade delay={0.2} inView>
          <Card className="w-full max-w-[420px] border-border/50 bg-background/80 px-2 py-2 backdrop-blur-sm sm:px-4 sm:py-4">
            <CardContent className="flex flex-col items-center px-6 py-8 text-center sm:px-8">
              {state === "verifying" && (
                <>
                  <div className="mb-4 rounded-full bg-primary/10 p-3">
                    <Loader2 className="h-8 w-8 animate-spin text-primary" />
                  </div>
                  <h2 className="mb-2 text-xl font-semibold">Verifying your email</h2>
                  <p className="text-muted-foreground">
                    Please wait while we verify your email address...
                  </p>
                </>
              )}

              {state === "success" && (
                <>
                  <div className="mb-4 rounded-full bg-green-500/10 p-3">
                    <CheckCircle2 className="h-8 w-8 text-green-500" />
                  </div>
                  <h2 className="mb-2 text-xl font-semibold">Email verified!</h2>
                  <p className="mb-6 text-muted-foreground">
                    Your email has been verified successfully. You'll be redirected to sign in
                    shortly.
                  </p>
                  <Button onClick={() => router.push("/login")} className="w-full">
                    Continue to Sign In
                  </Button>
                </>
              )}

              {state === "error" && (
                <>
                  <div className="mb-4 rounded-full bg-destructive/10 p-3">
                    <XCircle className="h-8 w-8 text-destructive" />
                  </div>
                  <h2 className="mb-2 text-xl font-semibold">Verification failed</h2>
                  <p className="mb-6 text-muted-foreground">
                    {error || "We couldn't verify your email. The link may be invalid."}
                  </p>
                  <div className="space-y-3 w-full">
                    <Button variant="outline" className="w-full" onClick={handleResendEmail}>
                      <Mail className="mr-2 h-4 w-4" />
                      Request new verification link
                    </Button>
                    <Link href="/signup" className="block">
                      <Button variant="ghost" className="w-full">
                        Back to Sign Up
                      </Button>
                    </Link>
                  </div>
                </>
              )}

              {state === "expired" && (
                <>
                  <div className="mb-4 rounded-full bg-yellow-500/10 p-3">
                    <XCircle className="h-8 w-8 text-yellow-600" />
                  </div>
                  <h2 className="mb-2 text-xl font-semibold">Link expired</h2>
                  <p className="mb-6 text-muted-foreground">
                    This verification link has expired. Please request a new one.
                  </p>
                  <div className="space-y-3 w-full">
                    <Button className="w-full" onClick={handleResendEmail}>
                      <Mail className="mr-2 h-4 w-4" />
                      Send new verification link
                    </Button>
                    <Link href="/login" className="block">
                      <Button variant="ghost" className="w-full">
                        Back to Sign In
                      </Button>
                    </Link>
                  </div>
                </>
              )}
            </CardContent>
          </Card>
        </BlurFade>
      </div>
    </div>
  );
}

function VerifyEmailLoading() {
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
        </div>
        <Card className="w-full max-w-[420px] border-border/50 bg-background/80 px-2 py-2 backdrop-blur-sm sm:px-4 sm:py-4">
          <CardContent className="flex items-center justify-center px-6 py-8 sm:px-8">
            <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
          </CardContent>
        </Card>
      </div>
    </div>
  );
}

export default function VerifyEmailPage() {
  return (
    <Suspense fallback={<VerifyEmailLoading />}>
      <VerifyEmailContent />
    </Suspense>
  );
}
