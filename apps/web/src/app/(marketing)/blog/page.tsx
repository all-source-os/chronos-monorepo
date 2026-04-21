import Link from "next/link";
import BlogCard from "@/components/blog-card";
import { BLOG_CATEGORIES, getBlogPosts } from "@/lib/blog";
import { siteConfig } from "@/lib/config";
import { cn, constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Blog — Engineering, AI Agents, and Event Sourcing",
  description:
    "Technical articles on event sourcing, AI agent memory, time-travel queries, and building with AllSource. Written by the team behind 469K events/sec Rust infrastructure.",
  canonical: "/blog",
});

export default async function Blog({
  searchParams,
}: {
  searchParams: Promise<{ author?: string; category?: string }>;
}) {
  const { author, category } = await searchParams;
  const allPosts = await getBlogPosts();

  let articles = allPosts.sort((a, b) => b.publishedAt.localeCompare(a.publishedAt));
  if (author) {
    articles = articles.filter((p) => p.author === author);
  }
  if (category) {
    articles = articles.filter((p) => p.category === category);
  }

  const activeLabel = category
    ? BLOG_CATEGORIES.find((c) => c.value === category)?.label
    : author
      ? `by ${author}`
      : null;

  return (
    <>
      <div className="relative mx-auto w-full max-w-screen-xl px-2.5 lg:px-20 mt-24">
        {/* Background glow */}
        <div className="pointer-events-none absolute inset-0 -top-32 flex items-start justify-center">
          <div className="h-[300px] w-[600px] rounded-full bg-primary/15 blur-[120px]" />
        </div>
        <div className="relative text-center py-16">
          <h1 className="text-3xl font-bold text-foreground sm:text-4xl">
            {activeLabel ? `Articles: ${activeLabel}` : "Articles"}
          </h1>
          <p className="mt-4 text-xl text-muted-foreground">
            {activeLabel ? (
              <a href="/blog" className="text-primary hover:underline">
                View all articles
              </a>
            ) : (
              `Latest news and updates from ${siteConfig.name}`
            )}
          </p>

          {/* Category tabs */}
          <div className="mt-8 flex items-center justify-center gap-2 flex-wrap">
            <Link
              href="/blog"
              className={cn(
                "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                !category && !author
                  ? "bg-primary text-primary-foreground"
                  : "bg-muted text-muted-foreground hover:text-foreground"
              )}
            >
              All
            </Link>
            {BLOG_CATEGORIES.map((cat) => (
              <Link
                key={cat.value}
                href={`/blog?category=${cat.value}`}
                className={cn(
                  "rounded-full px-4 py-1.5 text-sm font-medium transition-colors",
                  category === cat.value
                    ? "bg-primary text-primary-foreground"
                    : "bg-muted text-muted-foreground hover:text-foreground"
                )}
              >
                {cat.label}
              </Link>
            ))}
          </div>
        </div>
      </div>
      <div className="min-h-[50vh]">
        <div className="mx-auto grid w-full max-w-screen-xl grid-cols-1 gap-8 px-2.5 py-10 lg:px-20 lg:grid-cols-3">
          {articles.map((data, idx) => (
            <BlogCard key={data.slug} data={data} priority={idx <= 1} />
          ))}
        </div>
      </div>
    </>
  );
}
