import fs from "node:fs";
import path from "node:path";
import rehypePrettyCode from "rehype-pretty-code";
import rehypeStringify from "rehype-stringify";
import remarkGfm from "remark-gfm";
import remarkParse from "remark-parse";
import remarkRehype from "remark-rehype";
import { unified } from "unified";
import { siteConfig } from "@/lib/config";

export type BlogCategory = "engineering" | "use-cases" | "product";

export const BLOG_CATEGORIES: { value: BlogCategory; label: string }[] = [
  { value: "engineering", label: "Engineering" },
  { value: "use-cases", label: "Use Cases" },
  { value: "product", label: "Product" },
];

export type Post = {
  title: string;
  publishedAt: string;
  summary: string;
  author: string;
  slug: string;
  image?: string;
  category?: string;
};

function parseFrontmatter(fileContent: string) {
  const frontmatterRegex = /---\s*([\s\S]*?)\s*---/;
  const match = frontmatterRegex.exec(fileContent);

  if (!match || !match[1]) {
    return { data: {} as Post, content: fileContent };
  }

  const frontMatterBlock = match[1];
  const content = fileContent.replace(frontmatterRegex, "").trim();
  const frontMatterLines = frontMatterBlock.trim().split("\n");
  const metadata: Partial<Post> = {};

  for (const line of frontMatterLines) {
    const [key, ...valueArr] = line.split(": ");
    if (!key) continue;

    let value = valueArr.join(": ").trim();
    value = value.replace(/^['"](.*)['"]$/, "$1"); // Remove quotes
    metadata[key.trim() as keyof Post] = value;
  }

  return { data: metadata as Post, content };
}

function getMDXFiles(dir: string) {
  return fs.readdirSync(dir).filter((file) => path.extname(file) === ".mdx");
}

export async function markdownToHTML(markdown: string) {
  const p = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkRehype)
    .use(rehypePrettyCode, {
      // https://rehype-pretty.pages.dev/#usage
      theme: {
        light: "min-light",
        dark: "min-dark",
      },
      keepBackground: false,
    })
    .use(rehypeStringify)
    .process(markdown);

  return p.toString();
}

export async function getPost(slug: string) {
  // Sanitize slug to prevent path traversal: take only the filename component
  const sanitizedSlug = path.basename(slug);
  const filePath = path.join("content", `${sanitizedSlug}.mdx`);
  // Verify the resolved path is still within the content directory
  const contentDir = path.resolve(process.cwd(), "content");
  const resolvedPath = path.resolve(process.cwd(), filePath);
  if (!resolvedPath.startsWith(contentDir + path.sep)) {
    throw new Error(`Invalid blog slug: ${slug}`);
  }
  // Missing post → return null so callers can render a 404 instead of throwing
  // (which Next.js surfaces as an opaque 500). fs.readFileSync throws ENOENT
  // for unknown slugs, so probe existence first.
  if (!fs.existsSync(resolvedPath)) {
    return null;
  }
  const source = fs.readFileSync(filePath, "utf-8");
  const { content: rawContent, data: metadata } = parseFrontmatter(source);
  const content = await markdownToHTML(rawContent);
  const defaultImage = `${siteConfig.url}/og?title=${encodeURIComponent(metadata.title)}`;
  return {
    source: content,
    metadata: {
      ...metadata,
      image: metadata.image || defaultImage,
    },
    slug,
  };
}

async function getAllPosts(dir: string) {
  const mdxFiles = getMDXFiles(dir);
  return Promise.all(
    mdxFiles.map(async (file) => {
      const slug = path.basename(file, path.extname(file));
      const post = await getPost(slug);
      // getPost only returns null for missing files; these come straight from
      // the directory listing so they always exist, but narrow for the type checker.
      if (!post) {
        throw new Error(`Blog post disappeared during read: ${slug}`);
      }
      const { metadata, source } = post;
      return {
        ...metadata,
        slug,
        source,
      };
    })
  );
}

export async function getBlogPosts() {
  return getAllPosts(path.join(process.cwd(), "content"));
}
