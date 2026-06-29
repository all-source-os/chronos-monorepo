"use client";

import { buttonVariants, cn, Section } from "@allsource/ui";
import {
  AlertTriangle,
  ChevronRight,
  Cpu,
  Database,
  FileCheck,
  Layers,
  Radio,
  Thermometer,
  Timer,
  Zap,
} from "lucide-react";
import { motion } from "motion/react";
import Link from "next/link";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";

const features = [
  {
    title: "469K Events/sec Ingestion",
    description:
      "Purpose-built for high-throughput sensor data. Ingest hundreds of thousands of readings per second from device fleets, factory floors, and connected vehicles without backpressure.",
    icon: Zap,
    color: "from-orange-500/20 to-orange-500/5",
  },
  {
    title: "WAL Durability — Zero Data Loss",
    description:
      "Every sensor reading is written to the Write-Ahead Log with CRC32 checksums and configurable fsync. Power failures, crashes, network partitions — no reading is ever lost.",
    icon: Database,
    color: "from-amber-500/20 to-amber-500/5",
  },
  {
    title: "Time-Series Event Queries",
    description:
      "Query sensor readings by time range, device ID, reading type, or any combination. Sub-microsecond indexed lookups make real-time dashboards trivial.",
    icon: Timer,
    color: "from-yellow-500/20 to-yellow-500/5",
  },
  {
    title: "Batch Ingestion API",
    description:
      "Send thousands of readings in a single HTTP request. Edge gateways can buffer offline readings and sync in bulk when connectivity returns. Idempotent by design.",
    icon: Layers,
    color: "from-red-500/20 to-red-500/5",
  },
  {
    title: "Schema Validation per Device Type",
    description:
      "Define schemas for each sensor type — temperature, pressure, vibration, GPS. Invalid readings are rejected at ingestion, not discovered days later in a dashboard.",
    icon: FileCheck,
    color: "from-rose-500/20 to-rose-500/5",
  },
  {
    title: "Anomaly Detection Projections",
    description:
      "Define projections that track rolling averages, standard deviations, and threshold breaches. Get real-time alerts when a sensor reading deviates from expected ranges.",
    icon: AlertTriangle,
    color: "from-pink-500/20 to-pink-500/5",
  },
];

export default function IotTelemetryPage() {
  return (
    <>
      <Header />
      <main className="relative overflow-hidden">
        {/* Hero */}
        <Section className="relative pt-24 pb-16 text-center">
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6 }}
          >
            <span className="inline-flex items-center gap-2 rounded-full border bg-background/50 px-4 py-1.5 text-sm backdrop-blur-sm">
              <Thermometer className="h-4 w-4 text-orange-400" />
              IoT & Telemetry
            </span>
            <h1 className="mt-6 text-4xl font-bold tracking-tight sm:text-6xl">
              469K events per second.
              <br />
              <span className="bg-gradient-to-r from-orange-400 to-amber-400 bg-clip-text text-transparent">
                Every sensor. Every reading.
              </span>
            </h1>
            <p className="mx-auto mt-6 max-w-2xl text-lg text-muted-foreground">
              High-throughput ingestion with WAL durability for industrial IoT,
              connected vehicles, and device fleets. Time-series queries and
              anomaly detection projections — no separate TSDB needed.
            </p>
            <div className="mt-8 flex items-center justify-center gap-4">
              <Link
                href="/signup"
                className={cn(buttonVariants({ size: "lg" }))}
              >
                Start free trial
                <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link
                href="/docs/api"
                className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
              >
                API reference
              </Link>
            </div>

            {/* Key Metrics */}
            <motion.div
              className="mx-auto mt-12 grid max-w-3xl grid-cols-2 gap-4 md:grid-cols-4"
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{ duration: 0.5, delay: 0.3 }}
            >
              {[
                { value: "469K/s", label: "Ingestion Rate" },
                { value: "11.9us", label: "Query Latency" },
                { value: "0", label: "Readings Lost" },
                { value: "Snappy", label: "Compression" },
              ].map((metric) => (
                <div
                  key={metric.label}
                  className="rounded-xl border bg-background/50 p-4 backdrop-blur-sm"
                >
                  <div className="text-2xl font-bold text-orange-400">
                    {metric.value}
                  </div>
                  <div className="text-sm text-muted-foreground">
                    {metric.label}
                  </div>
                </div>
              ))}
            </motion.div>
          </motion.div>
        </Section>

        {/* Features */}
        <Section className="pb-16">
          <h2 className="mb-12 text-center text-3xl font-bold">
            Built for the firehose
          </h2>
          <div className="grid gap-6 sm:grid-cols-2 lg:grid-cols-3">
            {features.map((feature, i) => (
              <motion.div
                key={feature.title}
                initial={{ opacity: 0, y: 20 }}
                whileInView={{ opacity: 1, y: 0 }}
                transition={{ delay: i * 0.08 }}
                viewport={{ once: true }}
                className="rounded-xl border p-6"
              >
                <div
                  className={cn(
                    "mb-4 flex h-10 w-10 items-center justify-center rounded-lg bg-gradient-to-br",
                    feature.color,
                  )}
                >
                  <feature.icon className="h-5 w-5" />
                </div>
                <h3 className="mb-2 font-semibold">{feature.title}</h3>
                <p className="text-sm text-muted-foreground">
                  {feature.description}
                </p>
              </motion.div>
            ))}
          </div>
        </Section>

        {/* Code Example */}
        <Section className="pb-16">
          <h2 className="mb-4 text-center text-3xl font-bold">
            Batch ingest sensor readings
          </h2>
          <p className="mb-8 text-center text-muted-foreground">
            Send thousands of readings in a single request from edge gateways
          </p>
          <div className="mx-auto max-w-3xl">
            <div className="overflow-hidden rounded-xl border">
              <div className="flex items-center gap-2 bg-neutral-900 px-4 py-3">
                <div className="h-3 w-3 rounded-full bg-red-500" />
                <div className="h-3 w-3 rounded-full bg-yellow-500" />
                <div className="h-3 w-3 rounded-full bg-green-500" />
                <span className="ml-4 font-mono text-sm text-neutral-400">
                  sensor-ingest.sh
                </span>
              </div>
              <pre className="overflow-x-auto bg-neutral-950 p-6 text-sm leading-relaxed text-green-400">
{`# Batch ingest sensor readings from an edge gateway
curl -s -X POST https://api.all-source.xyz/api/v1/events \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{
    "events": [
      {
        "event_type": "sensor.temperature",
        "entity_id": "factory-12/line-3/motor-7",
        "data": {"celsius": 87.3, "unit": "C"},
        "timestamp": "2026-04-16T14:30:01.123Z"
      },
      {
        "event_type": "sensor.vibration",
        "entity_id": "factory-12/line-3/motor-7",
        "data": {"mm_per_sec": 4.7, "axis": "x"},
        "timestamp": "2026-04-16T14:30:01.124Z"
      },
      {
        "event_type": "sensor.temperature",
        "entity_id": "factory-12/line-3/motor-8",
        "data": {"celsius": 42.1, "unit": "C"},
        "timestamp": "2026-04-16T14:30:01.125Z"
      }
    ]
  }'

# {"accepted": 3, "rejected": 0}

# Query temperature readings for a specific motor over the last hour
curl -s https://api.all-source.xyz/api/v1/events/query \\
  -H "Authorization: Bearer $API_KEY" \\
  -d '{
    "entity_id": "factory-12/line-3/motor-7",
    "event_type": "sensor.temperature",
    "start_time": "2026-04-16T13:30:00Z",
    "end_time": "2026-04-16T14:30:00Z"
  }'

# {"events": [...], "count": 3600}  — one reading per second, all durable`}
              </pre>
            </div>
          </div>
        </Section>

        {/* CTA */}
        <Section className="pb-24 text-center">
          <Radio className="mx-auto mb-4 h-12 w-12 text-orange-400" />
          <h2 className="mb-4 text-3xl font-bold">
            Your sensors deserve a real event store
          </h2>
          <p className="mx-auto mb-8 max-w-xl text-muted-foreground">
            Stop shoehorning telemetry into time-series databases that were not
            built for event sourcing. Get durability, time-travel, and anomaly
            detection in one engine.
          </p>
          <div className="flex items-center justify-center gap-4">
            <Link
              href="/signup"
              className={cn(buttonVariants({ size: "lg" }))}
            >
              Start free trial
              <ChevronRight className="ml-1 h-4 w-4" />
            </Link>
            <Link
              href="/solutions/real-time-analytics"
              className={cn(buttonVariants({ variant: "outline", size: "lg" }))}
            >
              Real-time analytics
            </Link>
          </div>
          <div className="mt-6 flex items-center justify-center gap-6 text-sm text-muted-foreground">
            <Link href="/docs" className="underline">
              Documentation
            </Link>
            <Link href="/docs/api" className="underline">
              API Reference
            </Link>
            <Link
              href="https://github.com/all-source-os/all-source"
              className="underline"
            >
              GitHub
            </Link>
          </div>
        </Section>
      </main>
      <Footer />
    </>
  );
}
