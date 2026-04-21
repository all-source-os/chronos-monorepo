import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Financial Services — Transaction Logs with Temporal Consistency",
  description:
    "Immutable transaction history for banking, payments, and trading. Time-travel any account balance. Complete audit trail with cryptographic integrity. SOC2-ready.",
  canonical: "/solutions/financial-services",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
