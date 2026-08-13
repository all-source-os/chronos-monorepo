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
  /** Retained for former BlurFade call sites; static sections need no observer. */
  inView?: boolean;
}) {
  return <div className={cn(className)}>{children}</div>;
}
