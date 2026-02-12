import type { MetadataRoute } from "next";
import { headers } from "next/headers";
import { getBlogPosts } from "@/lib/blog";

export default async function sitemap(): Promise<MetadataRoute.Sitemap> {
  const allPosts = await getBlogPosts();
  const headersList = await headers();
  const domain = headersList.get("host") as string;
  const protocol = "https";

  return [
    {
      url: `${protocol}://${domain}`,
      lastModified: new Date(),
    },
    ...allPosts.map((post) => ({
      url: `${protocol}://${domain}/blog/${post.slug}`,
      lastModified: new Date(),
    })),
  ];
}
