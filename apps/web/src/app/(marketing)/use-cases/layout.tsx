import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Use Cases: Audit, Replay, Agent Memory",
  description:
    "See how AllSource supports audit trails, event replay, AI-agent memory with provenance, and financial transaction history — plus when not to use it.",
  canonical: "/use-cases",
});

export default function UseCasesLayout({ children }: { children: React.ReactNode }) {
  return children;
}
