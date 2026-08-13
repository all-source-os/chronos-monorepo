import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

// The only marketing route that shipped without metadata — it inherited the
// root title/description, so every answer engine saw an untitled duplicate of
// the homepage rather than a status page.
export const metadata: Metadata = constructMetadata({
  title: "Status — live AllSource service health",
  description:
    "Live availability for the AllSource event store, control plane, and hosted MCP endpoints. Check current uptime, recent incidents, and per-service health before you deploy.",
  canonical: "/status",
});

interface StatusLayoutProps {
  children: React.ReactNode;
}

export default async function Layout({ children }: StatusLayoutProps) {
  return children;
}
