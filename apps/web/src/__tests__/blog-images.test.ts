import { describe, expect, it } from "vitest";
// Reuse the exact check that gates the production build, so local/CI failures
// match what would block a Vercel deploy.
import { checkBlogImages } from "../../scripts/check-blog-images.mjs";

describe("blog image gate", () => {
  it("every blog post declares an image whose asset exists", () => {
    const errors = checkBlogImages();
    // Surface the offending posts in the failure message.
    expect(errors, `\n${errors.join("\n")}\n`).toEqual([]);
  });
});
