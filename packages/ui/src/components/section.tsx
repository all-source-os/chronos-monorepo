import type { ReactNode } from "react";
import { cn } from "../lib/utils";

interface SectionProps {
  id?: string;
  title?: string;
  subtitle?: string;
  description?: string;
  children?: ReactNode;
  className?: string;
  headingLevel?: 1 | 2;
}

export function Section({
  id,
  title,
  subtitle,
  description,
  children,
  className,
  headingLevel = 2,
}: SectionProps) {
  const Heading = headingLevel === 1 ? "h1" : "h2";

  return (
    <section id={id} className={cn("py-16 md:py-24", className)}>
      {(subtitle || title || description) && (
        <div className="flex flex-col items-center justify-center space-y-4 text-center mb-12">
          {subtitle && <span className="text-sm font-medium text-primary">{subtitle}</span>}
          {title && (
            <Heading className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
              {title}
            </Heading>
          )}
          {description && (
            <p className="mx-auto max-w-[700px] text-muted-foreground md:text-lg">{description}</p>
          )}
        </div>
      )}
      {children}
    </section>
  );
}

export default Section;
