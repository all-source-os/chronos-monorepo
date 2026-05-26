import { Suspense } from "react";
import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";
import { ConnectClient } from "./connect-client";

export const metadata = constructMetadata({
  title: "Connect to Claude Desktop",
  description: `Mint an API key and copy a ready-to-paste config to wire ${siteConfig.name} Prime into Claude Desktop.`,
});

export default function ConnectPage() {
  return (
    <div className="mx-auto w-full max-w-screen-md px-4 lg:px-8 py-24">
      {/* Suspense boundary is required by Next.js because ConnectClient
          reads URL search params (`source`, `key_name`, `return`) — see
          bead t-baff for the deep-link param contract. */}
      <Suspense fallback={null}>
        <ConnectClient />
      </Suspense>
    </div>
  );
}
