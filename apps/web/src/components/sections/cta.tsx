"use client";

import { buttonVariants, cn, Icons, Section } from "@allsource/ui";
import { CheckCircle2, Loader2 } from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import { useState } from "react";

export default function CtaSection() {
  const [email, setEmail] = useState("");
  const [status, setStatus] = useState<"idle" | "loading" | "success" | "error">("idle");
  const [errorMessage, setErrorMessage] = useState("");

  const handleWaitlist = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!email.trim()) return;

    setStatus("loading");
    setErrorMessage("");

    try {
      // Store waitlist signup — for now, redirect to signup with email prefilled
      const params = new URLSearchParams({ email: email.trim() });
      window.location.href = `/signup?${params.toString()}`;
    } catch {
      setStatus("error");
      setErrorMessage("Something went wrong. Please try again.");
    }
  };

  return (
    <Section
      id="cta"
      title="Give your AI agents perfect memory"
      subtitle="Persistent, time-travelling memory powered by AllSource Prime. Free tier — no credit card required."
      className="relative overflow-hidden rounded-xl py-16"
    >
      {/* Animated gradient background */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/10 via-primary/5 to-purple-500/10" />

      {/* Animated border glow */}
      <div className="absolute inset-0 rounded-xl">
        <div className="absolute inset-[1px] rounded-xl bg-background/80 backdrop-blur-sm" />
        <div className="absolute inset-0 rounded-xl bg-gradient-to-r from-primary/50 via-purple-500/50 to-primary/50 opacity-20 animate-gradient-x bg-[length:200%_auto]" />
      </div>

      {/* Email collection form */}
      <motion.div
        className="relative z-10 flex w-full flex-col items-center pt-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.6 }}
      >
        {status === "success" ? (
          <div className="flex items-center gap-2 rounded-lg bg-green-500/10 px-4 py-3 text-sm text-green-600 dark:text-green-400">
            <CheckCircle2 className="h-4 w-4" />
            Redirecting to sign up...
          </div>
        ) : (
          <form onSubmit={handleWaitlist} className="flex w-full max-w-md flex-col gap-3 sm:flex-row">
            <input
              type="email"
              placeholder="you@example.com"
              value={email}
              onChange={(e) => setEmail(e.target.value)}
              required
              className="h-11 flex-1 rounded-lg border border-border bg-background/80 px-4 text-sm backdrop-blur-sm placeholder:text-muted-foreground focus:border-primary focus:outline-none focus:ring-1 focus:ring-primary"
            />
            <motion.button
              type="submit"
              disabled={status === "loading" || !email.trim()}
              whileHover={{ scale: 1.02 }}
              whileTap={{ scale: 0.98 }}
              className={cn(
                buttonVariants({ variant: "default" }),
                "h-11 gap-2 px-6 text-background disabled:opacity-50"
              )}
            >
              {status === "loading" ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <>
                  <Icons.logo className="h-4 w-4" />
                  Get Started Free
                </>
              )}
            </motion.button>
          </form>
        )}
        {errorMessage && (
          <p className="mt-2 text-sm text-destructive">{errorMessage}</p>
        )}
        <p className="mt-3 text-xs text-muted-foreground">No credit card required. 10K events/month free.</p>
      </motion.div>

      {/* Action links */}
      <motion.div
        className="relative z-10 flex flex-col w-full sm:flex-row items-center justify-center space-y-3 sm:space-y-0 sm:space-x-4 pt-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.6, delay: 0.1 }}
      >
        <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.98 }}>
          <Link
            href="/signup"
            className={cn(
              buttonVariants({ variant: "outline" }),
              "w-full sm:w-auto flex gap-2 px-8 transition-all duration-300 hover:border-primary/50 hover:bg-primary/5"
            )}
          >
            <Icons.logo className="h-5 w-5" />
            Sign Up with OAuth
          </Link>
        </motion.div>
        <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.98 }}>
          <Link
            href="https://github.com/all-source-os/all-source"
            className={cn(
              buttonVariants({ variant: "outline" }),
              "w-full sm:w-auto flex gap-2 px-8 transition-all duration-300 hover:border-primary/50 hover:bg-primary/5"
            )}
          >
            <Icons.github className="h-5 w-5" />
            Star on GitHub
          </Link>
        </motion.div>
      </motion.div>

      {/* Floating particles effect */}
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        {[...Array(6)].map((_, i) => (
          <motion.div
            key={i}
            className="absolute w-2 h-2 rounded-full bg-primary/30"
            initial={{
              x: `${20 + i * 15}%`,
              y: "100%",
            }}
            animate={{
              y: "-20%",
              opacity: [0, 1, 0],
            }}
            transition={{
              duration: 4 + i * 0.5,
              repeat: Infinity,
              delay: i * 0.8,
              ease: "linear",
            }}
          />
        ))}
      </div>
    </Section>
  );
}
