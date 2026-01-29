/** @type {import('next').NextConfig} */
const nextConfig = {
  output: "standalone",
  // Transpile monorepo packages and icon libraries for proper SSR bundling
  transpilePackages: ["@allsource/ui", "react-icons"],
  images: {
    remotePatterns: [{ hostname: "localhost" }, { hostname: "randomuser.me" }],
  },
};

export default nextConfig;
