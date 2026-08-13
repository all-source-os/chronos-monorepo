import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Audit & Compliance — Immutable Event History for Regulators",
  description:
    "Preserve ordered changes with integrity checks, reconstruct past state, trace provenance, and apply role-based access and policy enforcement.",
  canonical: "/solutions/audit-compliance",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
