import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Tenant Setup Guide — Create Tenants and API Keys Programmatically",
  description:
    "How to create AllSource tenants, mint scoped API keys, and configure agents for production. Self-service onboard endpoint, bootstrap key rotation, and best practices.",
  canonical: "/docs/tenant-setup",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
