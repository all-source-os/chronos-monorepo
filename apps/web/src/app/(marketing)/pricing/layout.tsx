import type { Metadata } from "next";
import Footer from "@/components/sections/footer";
import Header from "@/components/sections/header";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Pricing — pay for the events your agents write",
  description:
    "Self-host AllSource free (MIT), or go hosted from $19/mo. Indie, Studio, and Scale tiers ship metered x402 micropayment credits and explicit MCP read/write access. Enterprise is custom.",
  canonical: "/pricing",
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
