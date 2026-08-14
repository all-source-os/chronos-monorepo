import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Use Cases: Audit, Replay, Agent Memory",
  description:
    "See how one AllSource deployment supports audit trails, replay, agent memory, and financial history across tenant-scoped projects without an external database.",
  canonical: "/use-cases",
});

export default function UseCasesLayout({ children }: { children: React.ReactNode }) {
  return children;
}
