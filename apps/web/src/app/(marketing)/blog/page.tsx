import BlogCard from "@/components/blog-card";
import { getBlogPosts } from "@/lib/blog";
import { siteConfig } from "@/lib/config";
import { constructMetadata } from "@/lib/utils";

export const metadata = constructMetadata({
  title: "Blog",
  description: `Latest news and updates from ${siteConfig.name}.`,
});

export default async function Blog() {
  const allPosts = await getBlogPosts();

  const articles = await Promise.all(
    allPosts.sort((a, b) => b.publishedAt.localeCompare(a.publishedAt))
  );

  return (
    <>
      <div className="relative mx-auto w-full max-w-screen-xl px-2.5 lg:px-20 mt-24">
        {/* Background glow */}
        <div className="pointer-events-none absolute inset-0 -top-32 flex items-start justify-center">
          <div className="h-[300px] w-[600px] rounded-full bg-primary/15 blur-[120px]" />
        </div>
        <div className="relative text-center py-16">
          <h1 className="text-3xl font-bold text-foreground sm:text-4xl">Articles</h1>
          <p className="mt-4 text-xl text-muted-foreground">
            Latest news and updates from {siteConfig.name}
          </p>
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
