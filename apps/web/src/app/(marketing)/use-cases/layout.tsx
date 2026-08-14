import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Use Cases: Audit, Replay, Agent Memory",
  description:
    "See how Core durability and Query Service HTTP, realtime, analytics, and projection reads support audit, replay, agent memory, and financial history.",
  canonical: "/use-cases",
});

export default function UseCasesLayout({ children }: { children: React.ReactNode }) {
  return children;
}
