import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Multi-Tenant SaaS — Secure Isolation with RBAC and Policy Enforcement",
  description:
    "Event sourcing for SaaS platforms. Tenant isolation at the event level. RBAC with 4 roles and 7 permissions. Policy engine for custom authorization rules. Per-tenant quotas and billing.",
  canonical: "/solutions/multi-tenant-saas",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
