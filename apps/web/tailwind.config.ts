import type { Config } from "tailwindcss";

const config: Config = {
  content: [
    "./src/pages/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/components/**/*.{js,ts,jsx,tsx,mdx}",
    "./src/app/**/*.{js,ts,jsx,tsx,mdx}",
  ],
  theme: {
    extend: {
      colors: {
        allsource: {
          primary: "#3b82f6",
          secondary: "#8b5cf6",
          accent: "#06b6d4",
        },
      },
    },
  },
  plugins: [],
};
export default config;
