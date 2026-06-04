import { siteConfig } from "@/lib/config";

/**
 * Below-the-fold stat strip. Stats used to be the FIRST thing a visitor saw,
 * animating up from "0K / 0μs" — the opposite of the pitch on a slow paint.
 * They now live below the fold and render at their FINAL value on first paint:
 * this is a plain server component reading the formatted `display` strings from
 * siteConfig, so there is no zero in the initial DOM and nothing to hydrate.
 */
export default function StatStrip() {
  return (
    <section className="w-full px-4 py-10 sm:px-6 lg:px-8">
      <div className="mx-auto grid max-w-4xl grid-cols-2 gap-6 rounded-xl border border-border bg-card/50 px-6 py-8 sm:grid-cols-4">
        {siteConfig.stats.map((stat) => (
          <div key={stat.label} className="text-center">
            <div className="text-2xl font-bold text-primary sm:text-3xl">{stat.display}</div>
            <div className="text-xs text-muted-foreground sm:text-sm">{stat.label}</div>
          </div>
        ))}
      </div>
    </section>
  );
}
