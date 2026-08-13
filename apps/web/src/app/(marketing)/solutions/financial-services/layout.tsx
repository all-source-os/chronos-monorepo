import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Financial Services — Transaction Logs with Temporal Consistency",
  description:
    "Append transaction changes as ordered events, reconstruct account state at a past sequence, and trace a result to its source events.",
  canonical: "/solutions/financial-services",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
