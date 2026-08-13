import { buttonVariants, cn } from "@allsource/ui";
import { ChevronRight } from "lucide-react";
import type { Metadata } from "next";
import Link from "next/link";
import { BLOG_CATEGORIES, type BlogCategory, getBlogPosts, type Post } from "@/lib/blog";
import { breadcrumbSchema } from "@/lib/structured-data";
import { constructMetadata } from "@/lib/utils";

export const metadata: Metadata = constructMetadata({
  title: "Event Sourcing for AI Agents",
  description:
    "The complete guide to event sourcing for AI agents: durable memory that survives restarts, full provenance, time-travel queries, and 11.9μs recall. Plus every AllSource deep-dive, grouped by theme.",
  canonical: "/event-sourcing-for-ai-agents",
});

// Maps a post's free-form `category` frontmatter onto a known BlogCategory.
// Anything unrecognized falls into "engineering" so no post is ever dropped.
function normalizeCategory(raw?: string): BlogCategory {
  const value = (raw ?? "").replace(/['"]/g, "").trim().toLowerCase();
  if (value === "use-cases" || value === "product" || value === "engineering") {
    return value;
  }
  return "engineering";
}

const SECTION_BLURB: Record<BlogCategory, string> = {
  engineering: "How the durable event store is built — WAL, Parquet, recall, and the Rust core.",
  "use-cases": "Patterns for putting event-sourced memory to work in real agent systems.",
  product: "Product updates, releases, and the story behind AllSource and Prime.",
};

const COMPARISONS = [
  { slug: "mem0", name: "mem0" },
  { slug: "letta", name: "Letta" },
  { slug: "zep", name: "Zep" },
] as const;

export default async function EventSourcingForAiAgentsPage() {
  const posts = await getBlogPosts();

  // Group posts by normalized category, newest first within each group.
  const grouped = new Map<BlogCategory, Post[]>();
  for (const post of posts) {
    const cat = normalizeCategory(post.category);
    const list = grouped.get(cat) ?? [];
    list.push(post);
    grouped.set(cat, list);
  }
  for (const list of grouped.values()) {
    list.sort((a, b) => (a.publishedAt < b.publishedAt ? 1 : -1));
  }

  const breadcrumb = breadcrumbSchema([
    { name: "Home", path: "/" },
    { name: "Event Sourcing for AI Agents", path: "/event-sourcing-for-ai-agents" },
  ]);

  return (
    <>
      <script
        type="application/ld+json"
        // biome-ignore lint/security/noDangerouslySetInnerHtml: JSON-LD structured data requires dangerouslySetInnerHTML
        dangerouslySetInnerHTML={{ __html: JSON.stringify(breadcrumb) }}
      />
      <div className="relative overflow-hidden">
        <div className="mx-auto w-full max-w-3xl px-4 py-16 sm:px-6 lg:px-8">
          {/* Hero */}
          <header className="mb-12">
            <p className="text-sm font-medium text-primary">The pillar guide</p>
            <h1 className="mt-2 text-4xl font-bold tracking-tight sm:text-5xl">
              Event Sourcing for AI Agents
            </h1>
            <p className="mt-6 text-lg text-muted-foreground">
              Agents forget. A chat window scrolls past, a process restarts, and the context is
              gone. Event sourcing fixes that at the storage layer: every decision, observation, and
              message becomes an immutable event you can replay, query at any point in time, and
              recall in microseconds.
            </p>
          </header>

          {/* What it is */}
          <section className="prose dark:prose-invert max-w-none">
            <h2>What event sourcing gives an agent</h2>
            <p>
              Instead of overwriting state, you append events. The current state is a projection you
              derive from the log — and because the log is durable, the agent&rsquo;s memory
              survives restarts, crashes, and redeploys. With AllSource Core that log is a Rust
              write-ahead log (CRC32 checksums, configurable fsync) backed by columnar Parquet, with
              in-memory projections that answer recalls at <strong>11.9μs p99</strong> and ingest at{" "}
              <strong>469K events/sec</strong>.
            </p>
            <ul>
              <li>
                <strong>Durable memory</strong> — nothing is lost on restart; the WAL + Parquet
                store is the source of truth.
              </li>
              <li>
                <strong>Full provenance</strong> — replay the log to reconstruct exactly what the
                agent knew and when.
              </li>
              <li>
                <strong>Time-travel queries</strong> — ask for any entity&rsquo;s state{" "}
                <em>as_of</em> a past moment, first-class.
              </li>
              <li>
                <strong>MCP access</strong> — 55+ tenant-scoped tools, rising to 73 when fleet and
                administrative controls are enabled.
              </li>
            </ul>
          </section>

          {/* Comparisons */}
          <section className="mt-12">
            <h2 className="text-2xl font-bold">How AllSource compares</h2>
            <p className="mt-3 text-muted-foreground">
              Evaluating a managed memory layer? Here&rsquo;s how a durable event store stacks up
              against the popular agent-memory tools.
            </p>
            <div className="mt-6 grid gap-3 sm:grid-cols-3">
              {COMPARISONS.map((c) => (
                <Link
                  key={c.slug}
                  href={`/vs/${c.slug}`}
                  className="group rounded-xl border p-4 transition-colors hover:bg-muted/30"
                >
                  <span className="font-medium">AllSource vs {c.name}</span>
                  <ChevronRight className="ml-1 inline h-4 w-4 transition-transform group-hover:translate-x-1" />
                </Link>
              ))}
            </div>
          </section>

          {/* Deep dives — grouped by theme, pulled from the blog */}
          <section className="mt-12">
            <h2 className="text-2xl font-bold">Go deeper</h2>
            <p className="mt-3 text-muted-foreground">
              Every AllSource deep-dive, grouped by theme. New posts surface here automatically.
            </p>
            <div className="mt-6 space-y-10">
              {BLOG_CATEGORIES.map(({ value, label }) => {
                const list = grouped.get(value) ?? [];
                if (list.length === 0) return null;
                return (
                  <div key={value}>
                    <h3 className="text-lg font-semibold">{label}</h3>
                    <p className="mt-1 text-sm text-muted-foreground">{SECTION_BLURB[value]}</p>
                    <ul className="mt-4 space-y-3">
                      {list.map((post) => (
                        <li key={post.slug}>
                          <Link
                            href={`/blog/${post.slug}`}
                            className="group block rounded-lg border p-4 transition-colors hover:bg-muted/30"
                          >
                            <span className="font-medium group-hover:text-primary">
                              {post.title}
                            </span>
                            {post.summary && (
                              <span className="mt-1 block text-sm text-muted-foreground">
                                {post.summary}
                              </span>
                            )}
                          </Link>
                        </li>
                      ))}
                    </ul>
                  </div>
                );
              })}
            </div>
          </section>

          {/* CTA */}
          <section className="mt-16 rounded-xl border bg-muted/20 p-8 text-center">
            <h2 className="text-2xl font-bold">Give your agents memory that survives</h2>
            <p className="mt-4 text-sm text-muted-foreground">
              Start on the hosted Indie plan, or self-host the whole stack for free under
              Apache-2.0.
            </p>
            <div className="mt-6 flex flex-wrap items-center justify-center gap-4">
              <Link href="/pricing" className={cn(buttonVariants({ variant: "default" }))}>
                See pricing <ChevronRight className="ml-1 h-4 w-4" />
              </Link>
              <Link href="/blog" className={cn(buttonVariants({ variant: "outline" }))}>
                Read the blog
              </Link>
            </div>
          </section>
        </div>
      </div>
    </>
  );
}
