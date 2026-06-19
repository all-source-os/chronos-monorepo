// Publish gate: every blog post in content/*.mdx must declare an `image` in its
// frontmatter, and that image asset must exist under public/. Run as part of the
// build (`build` script) so a post without a hero image fails the Vercel build
// instead of shipping with a blank/generated fallback card.
//
// Plain Node (not Rust) on purpose: this runs inside Vercel's Node build step,
// where a Rust toolchain isn't available. Zero deps, ESM.
import fs from "node:fs";
import path from "node:path";

const CONTENT_DIR = path.join(process.cwd(), "content");
const PUBLIC_DIR = path.join(process.cwd(), "public");

/** @returns {string[]} list of human-readable errors (empty = all good). */
export function checkBlogImages() {
  const errors = [];

  if (!fs.existsSync(CONTENT_DIR)) {
    return [`content directory not found at ${CONTENT_DIR} (run from apps/web)`];
  }

  const posts = fs.readdirSync(CONTENT_DIR).filter((f) => f.endsWith(".mdx"));

  for (const file of posts) {
    const src = fs.readFileSync(path.join(CONTENT_DIR, file), "utf8");
    const match = /---\s*([\s\S]*?)\s*---/.exec(src);
    const fm = {};
    if (match) {
      for (const line of match[1].trim().split("\n")) {
        const i = line.indexOf(": ");
        if (i > 0) {
          fm[line.slice(0, i).trim()] = line
            .slice(i + 2)
            .trim()
            .replace(/^['"](.*)['"]$/, "$1");
        }
      }
    }

    const image = fm.image;
    if (!image) {
      errors.push(`${file}: missing 'image' frontmatter`);
      continue;
    }
    if (!image.startsWith("/")) {
      errors.push(`${file}: image '${image}' must be a site-absolute path (start with /)`);
      continue;
    }
    if (!fs.existsSync(path.join(PUBLIC_DIR, image))) {
      errors.push(`${file}: image asset not found at public${image}`);
    }
  }

  return errors;
}

// When invoked directly (build gate), fail the process on any violation.
if (import.meta.url === `file://${process.argv[1]}`) {
  const errors = checkBlogImages();
  if (errors.length > 0) {
    console.error("\n✗ Blog image gate failed — every post needs an existing hero image:\n");
    for (const e of errors) console.error(`  - ${e}`);
    console.error(
      '\nAdd `image: "/assets/blog/<slug>.webp"` to the frontmatter and commit the asset under apps/web/public/assets/blog/.\n'
    );
    process.exit(1);
  }
  const count = fs.readdirSync(CONTENT_DIR).filter((f) => f.endsWith(".mdx")).length;
  console.log(`✓ Blog image gate: all ${count} posts have an existing hero image.`);
}
