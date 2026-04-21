import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Use Cases — Audit Trails, Event Replay, AI Agents, Finance",
  description:
    "Four production use cases where mutable databases fail and an event store is the right foundation. Audit trails, event replay, AI agent memory, and financial transaction history.",
  canonical: "/use-cases",
});

export default function UseCasesLayout({ children }: { children: React.ReactNode }) {
  return children;
}
