import { describe, expect, it } from "vitest";
import { constructMetadata } from "@/lib/utils";

describe("constructMetadata", () => {
  it("preserves the supplied document title", () => {
    const metadata = constructMetadata({
      title: "AllSource HTTP API",
      description: "HTTP API reference",
      canonical: "/docs/api",
    });

    expect(metadata.title).toBe("AllSource HTTP API");
    expect(metadata.openGraph).toMatchObject({ title: "AllSource HTTP API" });
    expect(metadata.twitter).toMatchObject({ title: "AllSource HTTP API" });
  });
});
