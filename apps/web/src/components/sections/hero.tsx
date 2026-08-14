import { buttonVariants, cn, Icons } from "@allsource/ui";
import Link from "next/link";
import HeroDemo from "@/components/sections/hero-demo";
import { indiePrice as defaultIndiePrice, siteConfig } from "@/lib/config";

function HeroPill() {
  return (
    <Link
      href="/what-is-allsource"
      className="flex w-fit items-center gap-2 rounded-full border border-border bg-card px-3 py-1.5 text-xs font-medium text-foreground transition-colors hover:border-primary/50 hover:text-primary sm:text-sm"
    >
      <span className="h-2 w-2 rounded-full bg-primary" aria-hidden="true" />
      What is AllSource Event Store?
      <span aria-hidden="true">→</span>
    </Link>
  );
}

function HeroTitles() {
  return (
    <div className="flex w-full flex-col gap-5 pt-8">
      <h1 className="text-balance text-4xl font-semibold leading-tight text-foreground sm:text-5xl lg:text-left lg:text-6xl">
        One event store for system history and AI memory.
      </h1>
      <p className="max-w-2xl text-balance text-lg leading-8 text-muted-foreground sm:text-xl lg:text-left">
        AllSource Event Store records decisions and state changes as immutable events. Core keeps
        durable history; Query Service separates HTTP, realtime, analytics, and projection reads;
        Prime gives agents cross-session memory backed by source events.
      </p>
    </div>
  );
}

function HeroCTA({ indiePrice }: { indiePrice: string }) {
  return (
    <>
      <div className="mt-8 flex w-full flex-col items-stretch gap-3 sm:flex-row sm:items-center lg:justify-start">
        <Link
          href="/signup"
          className={cn(
            buttonVariants({ variant: "default" }),
            "flex w-full gap-2 px-8 text-background sm:w-auto"
          )}
        >
          <Icons.logo className="h-5 w-5" />
          Start 14-day trial
        </Link>
        <Link
          href={siteConfig.links.github}
          className={cn(buttonVariants({ variant: "outline" }), "flex w-full gap-2 px-8 sm:w-auto")}
        >
          <Icons.github className="h-5 w-5" />
          Self-host on GitHub
        </Link>
        <Link
          href="/examples#capability-workbench"
          className={cn(buttonVariants({ variant: "ghost" }), "flex w-full gap-2 px-5 sm:w-auto")}
        >
          Explore demo →
        </Link>
      </div>
      <p className="mt-4 text-sm text-muted-foreground">
        Hosted plans from {indiePrice}/month after trial · Apache-2.0 Core available to self-host
      </p>
    </>
  );
}

export default function Hero({ indiePrice = defaultIndiePrice }: { indiePrice?: string }) {
  return (
    <section id="hero">
      <div className="mx-auto flex w-full max-w-7xl flex-col items-center gap-12 px-4 pt-32 sm:px-6 sm:pt-24 md:pt-32 lg:flex-row lg:items-center lg:justify-between lg:gap-16 lg:px-8">
        <div className="flex w-full flex-col items-center text-center lg:w-1/2 lg:items-start lg:text-left">
          <HeroPill />
          <HeroTitles />
          <HeroCTA indiePrice={indiePrice} />
        </div>

        <div className="flex w-full justify-center lg:w-1/2 lg:justify-end">
          <HeroDemo />
        </div>
      </div>
    </section>
  );
}
