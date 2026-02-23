/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  // Transpile monorepo packages and icon libraries for proper SSR bundling
  transpilePackages: ["@allsource/ui", "react-icons"],
  images: {
    remotePatterns: [{ hostname: "localhost" }, { hostname: "randomuser.me" }],
  },
  // OAuth proxy moved from rewrites (build-time) to a runtime API route at
  // src/app/api/v1/auth/oauth/[...path]/route.ts so CONTROL_PLANE_INTERNAL_URL
  // is read at request time, not baked in during the Vercel build.
};

export default nextConfig;
