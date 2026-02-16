"use client";

import { cn, FlickeringGrid, Ripple, Safari, Section } from "@allsource/ui";
import { motion } from "motion/react";

const features = [
  {
    title: "Immutable Event Log",
    description:
      "Every change is captured as an immutable event. Complete audit trails, perfect reproducibility, and the ability to reconstruct any past state with time-travel queries.",
    className: "hover:bg-red-500/10 transition-all duration-500 ease-out",
    content: (
      <>
        <Safari
          src={"/dashboard.png"}
          url="https://all-source.xyz"
          className="-mb-32 mt-4 max-h-64 w-full px-4 select-none drop-shadow-[0_0_28px_rgba(0,0,0,.1)] group-hover:translate-y-[-10px] transition-all duration-300"
        />
      </>
    ),
  },
  {
    title: "Sub-Microsecond Queries",
    description:
      "11.9μs p99 query latency with Parquet columnar storage. Lock-free data structures and zero-cost field access deliver real-time analytics at scale.",
    className: "order-3 xl:order-none hover:bg-blue-500/10 transition-all duration-500 ease-out",
    content: (
      <Safari
        src={"/dashboard.png"}
        url="https://all-source.xyz"
        className="-mb-32 mt-4 max-h-64 w-full px-4 select-none drop-shadow-[0_0_28px_rgba(0,0,0,.1)] group-hover:translate-y-[-10px] transition-all duration-300"
      />
    ),
  },
  {
    title: "AI-Native by Design",
    description:
      "27-tool MCP Server for Claude Desktop. AI agents can manage events, run analytics, detect anomalies, and orchestrate complex workflows autonomously.",
    className: "md:row-span-2 hover:bg-orange-500/10 transition-all duration-500 ease-out",
    content: (
      <>
        <FlickeringGrid
          className="z-0 absolute inset-0 [mask:radial-gradient(circle_at_center,#fff_400px,transparent_0)]"
          squareSize={4}
          gridGap={6}
          color="#000"
          maxOpacity={0.1}
          flickerChance={0.1}
          height={800}
          width={800}
        />
        <Safari
          src={"/dashboard.png"}
          url="https://all-source.xyz"
          className="-mb-48 ml-12 mt-16 h-full px-4 select-none drop-shadow-[0_0_28px_rgba(0,0,0,.1)] group-hover:translate-x-[-10px] transition-all duration-300"
        />
      </>
    ),
  },
  {
    title: "Stream Processing Pipelines",
    description:
      "Filter, Map, Reduce, Window, Branch, Enrich. Build real-time pipelines with a fluent API. Process 469K events/sec with the Rust core engine.",
    className:
      "flex-row order-4 md:col-span-2 md:flex-row xl:order-none hover:bg-green-500/10 transition-all duration-500 ease-out",
    content: (
      <>
        <Ripple className="absolute -bottom-full" />
        <Safari
          src={"/dashboard.png"}
          url="https://all-source.xyz"
          className="-mb-32 mt-4 max-h-64 w-full px-4 select-none drop-shadow-[0_0_28px_rgba(0,0,0,.1)] group-hover:translate-y-[-10px] transition-all duration-300"
        />
      </>
    ),
  },
];

export default function Component() {
  return (
    <Section
      title="The Solution"
      subtitle="Event Sourcing + AI Intelligence"
      description="AllSource combines high-performance event sourcing with AI-native tooling. Rust for speed, Go for orchestration, Elixir for distributed queries. The right tool for each job."
      className="bg-neutral-100 dark:bg-neutral-900"
    >
      <div className="mx-auto mt-16 grid max-w-sm grid-cols-1 gap-6 text-gray-500 md:max-w-3xl md:grid-cols-2 xl:grid-rows-2 md:grid-rows-3 xl:max-w-6xl xl:auto-rows-fr xl:grid-cols-3">
        {features.map((feature) => (
          <motion.div
            key={feature.title}
            className={cn(
              "group relative items-start overflow-hidden bg-neutral-50 dark:bg-neutral-800 p-6 rounded-2xl",
              feature.className
            )}
            initial={{ opacity: 0, y: 50 }}
            whileInView={{ opacity: 1, y: 0 }}
            transition={{
              duration: 0.5,
              type: "spring",
              stiffness: 100,
              damping: 30,
            }}
            viewport={{ once: true }}
          >
            <div>
              <h3 className="font-semibold mb-2 text-primary">{feature.title}</h3>
              <p className="text-foreground">{feature.description}</p>
            </div>
            {feature.content}
            <div className="absolute bottom-0 left-0 h-32 w-full bg-gradient-to-t from-neutral-50 dark:from-neutral-900 pointer-events-none" />
          </motion.div>
        ))}
      </div>
    </Section>
  );
}
