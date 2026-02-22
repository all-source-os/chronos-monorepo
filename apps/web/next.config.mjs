/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  // Transpile monorepo packages and icon libraries for proper SSR bundling
  transpilePackages: ["@allsource/ui", "react-icons"],
  images: {
    remotePatterns: [{ hostname: "localhost" }, { hostname: "randomuser.me" }],
  },
  async rewrites() {
    // Vercel can't reach Fly internal network, so this must be the public URL.
    // Locally, falls back to localhost.
    const controlPlaneUrl =
      process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
    return [
      {
        // Proxy OAuth endpoints to control plane so the browser-facing URL
        // stays on the frontend domain (where OAuth callbacks are registered).
        source: "/api/v1/auth/oauth/:path*",
        destination: `${controlPlaneUrl}/api/v1/auth/oauth/:path*`,
      },
    ];
  },
};

export default nextConfig;
