/** @type {import('next').NextConfig} */
const nextConfig = {
  // Docker deploys need standalone output. Vercel packages Next.js itself and
  // its post-build hook expects the normal `.next` trace layout.
  output: process.env.VERCEL ? undefined : "standalone",
  transpilePackages: ["@allsource/ui"],
};

export default nextConfig;
