import React from "react";
import { cn } from "../../lib/utils";

interface RippleProps {
  mainCircleSize?: number;
  mainCircleOpacity?: number;
  numCircles?: number;
  className?: string;
}

export const Ripple = React.memo(function Ripple({
  mainCircleSize = 210,
  mainCircleOpacity = 0.24,
  numCircles = 8,
  className,
}: RippleProps) {
  return (
    <div
      className={cn(
        "pointer-events-none select-none absolute inset-0 [mask-image:linear-gradient(to_bottom,white,transparent)]",
        className
      )}
    >
      {Array.from({ length: numCircles }, (_, i) => {
        const size = mainCircleSize + i * 70;
        const opacity = mainCircleOpacity - i * 0.03;
        const _animationDelay = `${i * 0.06}s`;
        const borderStyle = i === numCircles - 1 ? "dashed" : "solid";
        const borderOpacity = 5 + i * 5;

        return (
          <div
            // biome-ignore lint/suspicious/noArrayIndexKey: Ripple circles are generated based on index, index is appropriate
            key={i}
            className="absolute rounded-full bg-foreground/25 shadow-xl border"
            style={{
              width: `${size}px`,
              height: `${size}px`,
              opacity,
              animation: `ripple 2s ease ${i * 0.2}s infinite`,
              borderStyle,
              borderWidth: "1px",
              borderColor: `color-mix(in oklch, var(--foreground) ${borderOpacity}%, transparent)`,
              top: "50%",
              left: "50%",
              transform: "translate(-50%, -50%) scale(1)",
            }}
          />
        );
      })}
    </div>
  );
});

Ripple.displayName = "Ripple";

export default Ripple;
