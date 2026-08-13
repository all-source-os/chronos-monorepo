import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Architecture — the AI-agent suite on a durable event store",
  description:
    "An interactive C4 diagram of AllSource: a durable Rust event store (WAL + Parquet + DashMap) and the suite of AI-agent products built on top of it — Prime memory, prime-mcp, chronis, and the SDKs. Toggle between System Context and Container views.",
  canonical: "/architecture",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
