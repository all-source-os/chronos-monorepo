import { cn } from "@allsource/ui/utils";

/**
 * CSS-only replacement for `BlurFade` on the dashboard boot path.
 *
 * `BlurFade` is a `motion/react` component. Importing it anywhere in the
 * dashboard's initial graph pulls the whole animation runtime — ~134 KB of the
 * route's client JS — to fade some cards in. The dashboard is behind auth and
 * renders a "Loading…" shell first, so that cost is paid before the user can
 * see anything at all.
 *
 * This keeps the affordance (a staggered fade-and-rise) and drops the runtime,
 * using the `tailwindcss-animate` utilities already in the project.
 *
 * Behavioural difference, deliberate: `BlurFade` with `inView` waits until the
 * element scrolls into view; this animates on mount. On a dashboard whose
 * content sits at the top of the viewport that is equivalent in practice, and
 * it avoids shipping an IntersectionObserver wrapper to re-create it. Do not
 * swap this into long marketing pages where the in-view stagger is the point.
 */
export function FadeIn({
  children,
  className,
  delay = 0,
}: {
  children: React.ReactNode;
  /** Extra classes for the wrapper. */
  className?: string;
  /** Seconds before the animation starts, matching BlurFade's `delay` prop. */
  delay?: number;
}) {
  return (
    <div
      className={cn(
        "animate-in fade-in slide-in-from-bottom-2 fill-mode-both duration-500",
        className
      )}
      style={delay ? { animationDelay: `${delay}s` } : undefined}
    >
      {children}
    </div>
  );
}
