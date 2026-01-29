import { cn } from "../lib/utils";
import { ReactNode } from "react";

interface SectionProps {
  id?: string;
  title?: string;
  subtitle?: string;
  description?: string;
  children?: ReactNode;
  className?: string;
}

export function Section({ id, title, subtitle, description, children, className }: SectionProps) {
  return (
    <section id={id} className={cn("py-16 md:py-24", className)}>
      {(subtitle || title || description) && (
        <div className="flex flex-col items-center justify-center space-y-4 text-center mb-12">
          {subtitle && (
            <span className="text-sm font-medium text-primary">{subtitle}</span>
          )}
          {title && (
            <h2 className="text-3xl font-bold tracking-tighter sm:text-4xl md:text-5xl">
              {title}
            </h2>
          )}
          {description && (
            <p className="mx-auto max-w-[700px] text-muted-foreground md:text-lg">
              {description}
            </p>
          )}
        </div>
      )}
      {children}
    </section>
  );
}

export default Section;
