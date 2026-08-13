import { cn } from "@allsource/ui/utils";

/** Static dashboard section wrapper. Keeps layout stable during data refreshes. */
export function FadeIn({
  children,
  className,
}: {
  children: React.ReactNode;
  className?: string;
  /** Retained for call-site compatibility; dashboard sections no longer animate. */
  delay?: number;
}) {
  return <div className={cn(className)}>{children}</div>;
}
