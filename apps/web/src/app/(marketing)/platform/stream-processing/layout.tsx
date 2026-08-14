import type { Metadata } from "next";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "AllSource Stream Processing: Core Pipeline Operators",
  description:
    "Configure self-hosted AllSource Core pipelines with filter, map, reduce, window, and branch operators. See exact API shape, runtime, and current limits.",
  canonical: "/platform/stream-processing",
});

export default function Layout({ children }: { children: React.ReactNode }) {
  return children;
}
