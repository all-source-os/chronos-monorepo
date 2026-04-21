import type { Metadata, Viewport } from "next";
import { EarlyAccessBanner } from "@/components/early-access-banner";
import { TailwindIndicator } from "@/components/tailwind-indicator";
import { ThemeProvider } from "@/components/theme-provider";
import { ThemeToggle } from "@/components/theme-toggle";
import { cn, constructMetadata } from "@/lib/utils";
import "./globals.css";

export const metadata: Metadata = constructMetadata({
  title: "AllSource — AI-Native Event Store",
  description:
    "High-performance event sourcing with AI agent memory. 469K events/sec, 12μs queries. Time-travel, knowledge graphs, compressed index. Open source.",
  canonical: "/",
});

export const viewport: Viewport = {
  colorScheme: "dark",
  themeColor: [
    { media: "(prefers-color-scheme: dark)", color: "black" },
    { media: "(prefers-color-scheme: light)", color: "white" },
  ],
};

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <head>
        <link rel="preconnect" href="https://rsms.me/" />
        <link rel="stylesheet" href="https://rsms.me/inter/inter.css" />
        <script
          type="application/ld+json"
          dangerouslySetInnerHTML={{
            __html: JSON.stringify({
              "@context": "https://schema.org",
              "@type": "Organization",
              name: "AllSource",
              url: "https://all-source.xyz",
              logo: "https://all-source.xyz/logo.png",
              description:
                "AI-native event store for temporal data intelligence. 469K events/sec, 11.9us queries, 43 MCP tools.",
              sameAs: [
                "https://github.com/all-source-os/all-source",
                "https://twitter.com/allsourcedev",
              ],
              contactPoint: {
                "@type": "ContactPoint",
                email: "hello@all-source.xyz",
                contactType: "customer service",
              },
            }),
          }}
        />
      </head>
      <body className={cn("min-h-screen bg-background antialiased w-full mx-auto scroll-smooth")}>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
          <EarlyAccessBanner />
          {children}
          <ThemeToggle />
          <TailwindIndicator />
        </ThemeProvider>
      </body>
    </html>
  );
}
