import { buttonVariants, cn, Icons } from "@allsource/ui";
import Link from "next/link";
import { siteConfig } from "@/lib/config";

export default function CtaSection() {
  return (
    <section id="cta" className="mx-auto w-full max-w-7xl px-4 py-20 sm:px-6 lg:px-8">
      <div className="border border-border bg-card px-6 py-12 sm:px-10 lg:flex lg:items-end lg:justify-between lg:gap-12">
        <div className="max-w-2xl">
          <p className="font-mono text-xs font-semibold uppercase tracking-[0.2em] text-primary">
            Write → inspect → query
          </p>
          <h2 className="mt-4 text-balance text-3xl font-semibold text-foreground sm:text-4xl">
            Store one real event, then query it back.
          </h2>
          <p className="mt-4 text-pretty text-base leading-7 text-muted-foreground sm:text-lg">
            Start with hosted AllSource, or run the Apache-2.0 core on your own infrastructure. Both
            use the same event model and APIs.
          </p>
        </div>

        <div className="mt-8 flex shrink-0 flex-col gap-3 sm:flex-row lg:mt-0 lg:flex-col xl:flex-row">
          <Link
            href="/signup"
            className={cn(buttonVariants({ variant: "default" }), "gap-2 px-6 text-background")}
          >
            <Icons.logo className="h-4 w-4" />
            Start 14-day trial
          </Link>
          <Link
            href={siteConfig.links.github}
            className={cn(buttonVariants({ variant: "outline" }), "gap-2 px-6")}
          >
            <Icons.github className="h-4 w-4" />
            Run it yourself
          </Link>
        </div>
      </div>
    </section>
  );
}
