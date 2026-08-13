import type { ComponentPropsWithoutRef } from "react";

type DecorativeMotionProps = {
  animate?: unknown;
  initial?: unknown;
  transition?: unknown;
  viewport?: unknown;
  whileHover?: unknown;
  whileInView?: unknown;
};

function staticProps<T extends object>(props: T & DecorativeMotionProps): T {
  const {
    animate: _animate,
    initial: _initial,
    transition: _transition,
    viewport: _viewport,
    whileHover: _whileHover,
    whileInView: _whileInView,
    ...htmlProps
  } = props;

  return htmlProps as T;
}

function StaticDiv(props: ComponentPropsWithoutRef<"div"> & DecorativeMotionProps) {
  return <div {...staticProps(props)} />;
}

function StaticHeading(props: ComponentPropsWithoutRef<"h1"> & DecorativeMotionProps) {
  return <h1 {...staticProps(props)} />;
}

function StaticParagraph(props: ComponentPropsWithoutRef<"p"> & DecorativeMotionProps) {
  return <p {...staticProps(props)} />;
}

/**
 * Drop-in subset for content pages that used Motion only to reveal already-static copy.
 * Keeps markup visible on first paint and avoids shipping Motion to those routes.
 */
export const staticMotion = {
  div: StaticDiv,
  h1: StaticHeading,
  p: StaticParagraph,
};
