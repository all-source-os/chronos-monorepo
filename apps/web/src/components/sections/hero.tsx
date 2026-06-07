"use client";

import { buttonVariants, cn, Icons } from "@allsource/ui";
import { motion } from "motion/react";
import Link from "next/link";
import HeroDemo from "@/components/sections/hero-demo";
import { indiePrice as defaultIndiePrice, siteConfig } from "@/lib/config";

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

const ease: [number, number, number, number] = [0.16, 1, 0.3, 1];

function HeroPill() {
  return (
    <motion.a
      href="/blog/introducing-allsource"
      className="flex w-fit items-center space-x-2 rounded-full bg-primary/20 px-2 py-1 ring-1 ring-accent whitespace-pre"
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.8, ease }}
    >
      <div className="w-fit rounded-full bg-accent px-2 py-0.5 text-center text-xs font-medium text-primary sm:text-sm">
        New
      </div>
      <p className="text-xs font-medium text-primary sm:text-sm">
        Your AI agent can now pay per call (x402, live in v0.19)
      </p>
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
          fill="currentColor"
        />
      </svg>
    </motion.a>
  );
}

function HeroTitles() {
  return (
    <div className="flex w-full flex-col space-y-4 pt-8">
      <motion.h1
        className="text-balance text-4xl font-semibold leading-tight text-foreground sm:text-5xl lg:text-left lg:text-6xl"
        initial={{ filter: "blur(10px)", opacity: 0, y: 30 }}
        animate={{ filter: "blur(0px)", opacity: 1, y: 0 }}
        transition={{ duration: 1, ease }}
      >
        Your agents already forget. <AnimatedGradientText>Stop letting them.</AnimatedGradientText>
      </motion.h1>
      <motion.p
        className="max-w-xl text-balance text-lg leading-8 text-muted-foreground sm:text-xl lg:text-left"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.4, duration: 0.8, ease }}
      >
        AllSource records every event your agent emits, with{" "}
        <span className="font-semibold text-primary">12μs recall</span> on every restart. Pay only
        when the agent reads.
      </motion.p>
    </div>
  );
}

function HeroCTA({ indiePrice }: { indiePrice: string }) {
  return (
    <>
      <motion.div
        className="mt-8 flex w-full flex-col items-stretch gap-4 sm:flex-row sm:items-center lg:justify-start"
        initial={{ opacity: 0, y: 20 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ delay: 0.6, duration: 0.8, ease }}
      >
        <motion.div
          whileHover={{ scale: 1.05 }}
          whileTap={{ scale: 0.98 }}
          className="group relative"
        >
          <Link
            href="/pricing"
            className={cn(
              buttonVariants({ variant: "default" }),
              "relative flex w-full gap-2 px-8 text-background sm:w-auto"
            )}
          >
            <Icons.logo className="h-5 w-5" />
            Start Indie — {indiePrice}
          </Link>
        </motion.div>
        <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.98 }}>
          <Link
            href={siteConfig.links.github}
            className={cn(
              buttonVariants({ variant: "outline" }),
              "flex w-full gap-2 px-8 transition-all duration-300 hover:border-primary/50 hover:bg-primary/5 sm:w-auto"
            )}
          >
            <Icons.github className="h-5 w-5" />
            Self-host on GitHub
          </Link>
        </motion.div>
      </motion.div>
      <motion.p
        className="mt-4 text-sm text-muted-foreground"
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ delay: 0.8, duration: 0.8 }}
      >
        Open source · MIT licensed · Self-host or cloud
      </motion.p>
    </>
  );
}

export default function Hero2({ indiePrice = defaultIndiePrice }: { indiePrice?: string }) {
  return (
    <section id="hero">
      <div className="relative mx-auto flex w-full max-w-7xl flex-col items-center gap-12 px-4 pt-32 sm:px-6 sm:pt-24 md:pt-32 lg:flex-row lg:items-center lg:justify-between lg:gap-16 lg:px-8">
        {/* Left half — the pitch */}
        <div className="flex w-full flex-col items-center text-center lg:w-1/2 lg:items-start lg:text-left">
          <HeroPill />
          <HeroTitles />
          <HeroCTA indiePrice={indiePrice} />
        </div>

        {/* Right half — the product working (self-contained, never blocks LCP) */}
        <motion.div
          className="flex w-full justify-center lg:w-1/2 lg:justify-end"
          initial={{ opacity: 0, y: 40 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ delay: 0.5, duration: 1, ease }}
        >
          <HeroDemo />
        </motion.div>

        <div className="pointer-events-none absolute inset-x-0 -bottom-12 h-1/3 bg-gradient-to-t from-background via-background to-transparent lg:h-1/4" />
      </div>
    </section>
  );
}
