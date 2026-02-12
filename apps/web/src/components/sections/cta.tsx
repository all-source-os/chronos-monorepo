"use client";

import { buttonVariants, cn, Icons, Section } from "@allsource/ui";
import { motion } from "motion/react";
import Link from "next/link";

export default function CtaSection() {
  return (
    <Section
      id="cta"
      title="Give your application perfect memory"
      subtitle="Query any point in history. Never lose an event. Free tier with 50K events/month."
      className="relative overflow-hidden rounded-xl py-16"
    >
      {/* Animated gradient background */}
      <div className="absolute inset-0 bg-gradient-to-br from-primary/10 via-primary/5 to-purple-500/10" />

      {/* Animated border glow */}
      <div className="absolute inset-0 rounded-xl">
        <div className="absolute inset-[1px] rounded-xl bg-background/80 backdrop-blur-sm" />
        <div className="absolute inset-0 rounded-xl bg-gradient-to-r from-primary/50 via-purple-500/50 to-primary/50 opacity-20 animate-gradient-x bg-[length:200%_auto]" />
      </div>

      {/* Content */}
      <motion.div
        className="relative z-10 flex flex-col w-full sm:flex-row items-center justify-center space-y-4 sm:space-y-0 sm:space-x-4 pt-4"
        initial={{ opacity: 0, y: 20 }}
        whileInView={{ opacity: 1, y: 0 }}
        viewport={{ once: true }}
        transition={{ duration: 0.6 }}
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
