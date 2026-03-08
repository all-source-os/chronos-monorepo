import type { Metadata, Viewport } from "next";
import { ThemeProvider } from "@/components/theme-provider";
import { cn } from "@allsource/ui/utils";
import "./globals.css";

export const metadata: Metadata = {
  title: "AllSource Admin",
  description: "AllSource platform administration dashboard",
};

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
      </head>
      <body className={cn("min-h-screen bg-background antialiased w-full mx-auto scroll-smooth")}>
        <ThemeProvider attribute="class" defaultTheme="dark" enableSystem={false}>
          {children}
        </ThemeProvider>
      </body>
    </html>
  );
}
