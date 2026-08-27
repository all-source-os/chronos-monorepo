import { buttonVariants, cn, Icons } from "@allsource/ui";
import { Menu } from "lucide-react";
import Link from "next/link";
import { siteConfig } from "@/lib/config";

const primaryNavigation = [
  { href: "/what-is-allsource", label: "Product map" },
  { href: "/platform/event-sourcing", label: "Event store" },
  { href: "/prime", label: "Agent memory" },
  { href: "/design-partners", label: "Design partners" },
  { href: "/use-cases", label: "Use cases" },
  { href: "/examples", label: "Demo" },
  { href: "/docs", label: "Docs" },
  { href: "/pricing", label: "Pricing" },
];

export default function Header() {
  return (
    <header className="sticky top-0 z-50 border-b border-border/80 bg-background/95 backdrop-blur">
      <div className="container mx-auto flex h-16 items-center justify-between gap-4">
        <Link
          href="/"
          title="AllSource home"
          className="flex shrink-0 items-center gap-2 rounded-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
        >
          <Icons.logo className="h-9 w-9" aria-hidden="true" />
          <span className="leading-tight">
            <span className="block text-lg font-semibold tracking-tight">{siteConfig.name}</span>
            <span className="block font-mono text-[10px] uppercase tracking-[0.16em] text-muted-foreground">
              Event Store
            </span>
          </span>
        </Link>

        <nav aria-label="Primary" className="hidden items-center gap-1 lg:flex">
          {primaryNavigation.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className="rounded-md px-3 py-2 text-sm font-medium text-muted-foreground transition-colors hover:bg-muted hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
            >
              {item.label}
            </Link>
          ))}
        </nav>

        <div className="hidden items-center gap-2 lg:flex">
          <Link href="/login" className={buttonVariants({ variant: "ghost" })}>
            Sign in
          </Link>
          <Link
            href="/signup"
            className={cn(buttonVariants({ variant: "default" }), "text-primary-foreground")}
          >
            Start 14-day trial
          </Link>
        </div>

        <details className="group relative lg:hidden">
          <summary className="flex h-10 cursor-pointer list-none items-center gap-2 rounded-md border border-border px-3 text-sm font-medium marker:content-none focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring">
            <Menu className="h-4 w-4" aria-hidden="true" />
            Menu
          </summary>
          <div className="absolute right-0 top-12 w-[min(20rem,calc(100vw-2rem))] rounded-xl border border-border bg-background p-2 shadow-xl">
            <nav aria-label="Mobile primary" className="grid gap-1">
              {primaryNavigation.map((item) => (
                <Link
                  key={item.href}
                  href={item.href}
                  className="rounded-lg px-3 py-2.5 text-sm font-medium hover:bg-muted focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
                >
                  {item.label}
                </Link>
              ))}
            </nav>
            <div className="mt-2 grid gap-2 border-t border-border pt-2">
              <Link href="/login" className={buttonVariants({ variant: "outline" })}>
                Sign in
              </Link>
              <Link
                href="/signup"
                className={cn(buttonVariants({ variant: "default" }), "text-primary-foreground")}
              >
                Start 14-day trial
              </Link>
            </div>
          </div>
        </details>
      </div>
    </header>
  );
}
