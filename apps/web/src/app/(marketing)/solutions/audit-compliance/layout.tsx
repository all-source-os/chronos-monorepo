import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Audit & Compliance — Immutable Event History for Regulators",
  description:
    "Complete audit trails with cryptographic integrity. Reconstruct any past state in seconds, not days. SOC2-ready event sourcing with RBAC, policy enforcement, and full provenance.",
  canonical: "/solutions/audit-compliance",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
