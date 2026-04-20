"use client";

import { buttonVariants, cn, HeroVideoDialog, Icons } from "@allsource/ui";
import { motion } from "motion/react";
import Link from "next/link";
import { useEffect, useRef, useState } from "react";

// Animated gradient text component for magic effect
function AnimatedGradientText({ children }: { children: React.ReactNode }) {
  return (
    <span className="relative inline-block">
      <span className="animate-gradient-x bg-gradient-to-r from-primary via-purple-500 to-primary bg-[length:200%_auto] bg-clip-text text-transparent">
        {children}
      </span>
    </span>
  );
}

// Count-up animation hook
function useCountUp(end: number, duration: number = 2000, delay: number = 0) {
  const [count, setCount] = useState(0);
  const [hasStarted, setHasStarted] = useState(false);
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const observer = new IntersectionObserver(
      ([entry]) => {
        if (entry?.isIntersecting && !hasStarted) {
          setHasStarted(true);
        }
      },
      { threshold: 0.5 }
    );

    if (ref.current) {
      observer.observe(ref.current);
    }

    return () => observer.disconnect();
  }, [hasStarted]);

  useEffect(() => {
    if (!hasStarted) return;

    const timeout = setTimeout(() => {
      const startTime = Date.now();
      const animate = () => {
        const elapsed = Date.now() - startTime;
        const progress = Math.min(elapsed / duration, 1);
        // Ease out cubic
        const eased = 1 - (1 - progress) ** 3;
        setCount(Math.floor(eased * end));

        if (progress < 1) {
          requestAnimationFrame(animate);
        }
      };
      requestAnimationFrame(animate);
    }, delay);

    return () => clearTimeout(timeout);
  }, [hasStarted, end, duration, delay]);

  return { count, ref };
}

const ease: [number, number, number, number] = [0.16, 1, 0.3, 1];

function HeroPill() {
  return (
    <motion.a
      href="/blog/introducing-allsource"
      className="flex w-auto items-center space-x-2 rounded-full bg-primary/20 px-2 py-1 ring-1 ring-accent whitespace-pre"
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.8, ease }}
    >
      <div className="w-fit rounded-full bg-accent px-2 py-0.5 text-center text-xs font-medium text-primary sm:text-sm">
        v0.17.0
      </div>
      <p className="text-xs font-medium text-primary sm:text-sm">Prime: Agent Memory Engine shipped</p>
      <svg
        width="12"
        height="12"
        className="ml-1"
        viewBox="0 0 12 12"
        fill="none"
        xmlns="http://www.w3.org/2000/svg"
        aria-hidden="true"
      >
        <path
          d="M8.78141 5.33312L5.20541 1.75712L6.14808 0.814453L11.3334 5.99979L6.14808 11.1851L5.20541 10.2425L8.78141 6.66645H0.666748V5.33312H8.78141Z"
          fill="hsl(var(--primary))"
        />
      </svg>
    </motion.a>
  );
}

function HeroTitles() {
  return (
    <div className="flex w-full max-w-3xl flex-col space-y-4 overflow-hidden pt-8">
      <motion.h1
        className="text-center text-4xl font-medium leading-tight text-foreground sm:text-5xl md:text-6xl"
        initial={{ filter: "blur(10px)", opacity: 0, y: 50 }}
        animate={{ filter: "blur(0px)", opacity: 1, y: 0 }}
        transition={{
          duration: 1,
          ease,
          staggerChildren: 0.2,
        }}
      >
        {["Give your", "AI agents"].map((text) => (
          <motion.span
            key={text}
            className="inline-block px-1 md:px-2 text-balance font-semibold"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              duration: 0.8,
              ease,
            }}
          >
            {text}
          </motion.span>
        ))}
        <motion.span
          className="inline-block px-1 md:px-2 text-balance font-semibold"
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{
            duration: 0.8,
            ease,
            delay: 0.3,
          }}
        >
          <AnimatedGradientText>perfect memory</AnimatedGradientText>
        </motion.span>
      </motion.h1>
      <motion.p
        className="mx-auto max-w-2xl text-center text-lg leading-7 text-muted-foreground sm:text-xl sm:leading-9 text-balance"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{
          delay: 0.6,
          duration: 0.8,
          ease,
        }}
      >
        AllSource Prime — the temporal intelligence platform for autonomous AI. Time-travel your
        data, query any past state, and give every agent{" "}
        <span className="text-primary font-semibold">persistent, provable memory</span>.
      </motion.p>
    </div>
  );
}

function CountUpStat({
  value,
  suffix,
  label,
  delay,
}: {
  value: number;
  suffix: string;
  label: string;
  delay: number;
}) {
  const { count, ref } = useCountUp(value, 2000, delay);

  return (
    <div ref={ref} className="text-center group">
      <motion.div
        className="text-2xl font-bold text-primary sm:text-3xl transition-transform group-hover:scale-110"
        whileHover={{ scale: 1.1 }}
      >
        {count.toLocaleString()}
        {suffix}
      </motion.div>
      <div className="text-xs text-muted-foreground sm:text-sm">{label}</div>
    </div>
  );
}

function HeroStats() {
  const stats = [
    { value: 469, suffix: "K", label: "events/sec", delay: 0 },
    { value: 11.9, suffix: "μs", label: "p99 latency", delay: 100 },
    { value: 27, suffix: "", label: "MCP tools", delay: 200 },
    { value: 129, suffix: "MB", label: "footprint", delay: 300 },
  ];

  return (
    <motion.div
      className="mx-auto mt-8 grid grid-cols-2 gap-4 sm:grid-cols-4 sm:gap-8"
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 0.7, duration: 0.8, ease }}
    >
      {stats.map((stat) => (
        <CountUpStat key={stat.label} {...stat} />
      ))}
    </motion.div>
  );
}

function HeroCTA() {
  return (
    <>
      <motion.div
        className="mx-auto mt-8 flex w-full max-w-2xl flex-col items-center justify-center space-y-4 sm:flex-row sm:space-x-4 sm:space-y-0"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.9, duration: 0.8, ease }}
      >
        <motion.div
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.98 }}
          className="relative group"
        >
          {/* Glow effect */}
          <div className="absolute -inset-1 bg-gradient-to-r from-primary via-purple-500 to-primary rounded-lg blur-lg opacity-0 group-hover:opacity-70 transition-opacity duration-500" />
          <Link
            href="/signup"
            className={cn(
              buttonVariants({ variant: "default" }),
              "relative w-full sm:w-auto text-background flex gap-2 px-8 transition-shadow duration-300 hover:shadow-lg hover:shadow-primary/25"
            )}
          >
            <Icons.logo className="h-5 w-5" />
            Start Your Project
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
            View on GitHub
          </Link>
        </motion.div>
      </motion.div>
      <motion.p
        className="mt-4 text-sm text-muted-foreground"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 1.1, duration: 0.8 }}
      >
        Open source · MIT licensed · Self-host or cloud
      </motion.p>
    </>
  );
}

function HeroImage() {
  return (
    <motion.div
      className="relative mx-auto flex w-full items-center justify-center mt-16"
      initial={{ opacity: 0, y: 50 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ delay: 1.3, duration: 1, ease }}
    >
      {/* Glow effect behind video */}
      <div className="absolute inset-0 flex items-center justify-center">
        <div className="h-[60%] w-[80%] max-w-screen-lg rounded-3xl bg-gradient-to-r from-primary/30 via-purple-500/20 to-primary/30 blur-3xl animate-pulse" />
      </div>

      <div className="relative group">
        {/* Hover glow intensification */}
        <div className="absolute -inset-4 rounded-2xl bg-gradient-to-r from-primary/20 via-purple-500/10 to-primary/20 blur-2xl opacity-0 group-hover:opacity-100 transition-opacity duration-500" />

        <HeroVideoDialog
          animationStyle="from-center"
          videoSrc="https://www.youtube.com/embed/qh3NGpYRG3I?si=4rb-zSdDkVK9qxxb"
          thumbnailSrc="/assets/hero-screenshot.png"
          thumbnailAlt="AllSource Dashboard — Event Explorer with real-time metrics"
          className="relative border rounded-lg shadow-lg shadow-primary/10 max-w-screen-lg transition-shadow duration-500 group-hover:shadow-xl group-hover:shadow-primary/20"
        />
      </div>
    </motion.div>
  );
}

export default function Hero2() {
  return (
    <section id="hero">
      <div className="relative flex w-full flex-col items-center justify-start px-4 pt-32 sm:px-6 sm:pt-24 md:pt-32 lg:px-8">
        <HeroPill />
        <HeroTitles />
        <HeroStats />
        <HeroCTA />
        <HeroImage />
        <div className="pointer-events-none absolute inset-x-0 -bottom-12 h-1/3 bg-gradient-to-t from-background via-background to-transparent lg:h-1/4" />
      </div>
    </section>
  );
}
