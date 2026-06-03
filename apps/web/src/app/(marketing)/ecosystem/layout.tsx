import type { Metadata } from "next";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Ecosystem — the apps your AI agent can use",
  description:
    "An interactive map of the public AllSource ecosystem for AI agents: durable memory over MCP (prime_* tools), the Claude Desktop DXT, the chronis task CLI, the SDKs, and the public event API — with copy-paste install, MCP config, and curl for each.",
  canonical: "/ecosystem",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return (
    <>
      <Header />
      <main>{children}</main>
      <Footer />
    </>
  );
}
