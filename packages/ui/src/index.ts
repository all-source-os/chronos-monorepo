// Utility exports
export { cn } from "./lib/utils";

// Component exports
export { Button, buttonVariants } from "./components/button";
export type { ButtonProps } from "./components/button";

export { Badge, badgeVariants } from "./components/badge";
export type { BadgeProps } from "./components/badge";

export {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from "./components/card";
export type { CardTitleProps } from "./components/card";

// Form components
export { Input } from "./components/input";

export { Label } from "./components/label";

export { Textarea } from "./components/textarea";
export type { TextareaProps } from "./components/textarea";

export { Switch } from "./components/switch";

// Layout components
export { Separator } from "./components/separator";
export type { SeparatorProps } from "./components/separator";

export { Skeleton } from "./components/skeleton";
export type { SkeletonProps } from "./components/skeleton";

// Interactive components
export {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "./components/accordion";

// Carousel
export {
  Carousel,
  CarouselContent,
  CarouselItem,
  CarouselNext,
  CarouselPrevious,
  useCarousel,
} from "./components/carousel";
export type { CarouselApi } from "./components/carousel";

// Drawer
export {
  Drawer,
  DrawerClose,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerOverlay,
  DrawerPortal,
  DrawerTitle,
  DrawerTrigger,
} from "./components/drawer";

// Navigation Menu
export {
  NavigationMenu,
  NavigationMenuContent,
  NavigationMenuIndicator,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
  NavigationMenuTrigger,
  NavigationMenuViewport,
  navigationMenuTriggerStyle,
} from "./components/navigation-menu";

// Section
export { Section } from "./components/section";

// Safari
export { Safari } from "./components/safari";
export type { SafariProps } from "./components/safari";

// Avatar Circles
export { AvatarCircles } from "./components/avatar-circles";

// Icons
export { Icons } from "./components/icons";
export type { IconKeys } from "./components/icons";

// Chart
export {
  ChartContainer,
  ChartTooltip,
  ChartTooltipContent,
  ChartLegend,
  ChartLegendContent,
  ChartStyle,
} from "./components/chart";
export type { ChartConfig } from "./components/chart";

// MagicUI components
export { BlurFade } from "./components/magicui/blur-fade";
export { BorderBeam } from "./components/magicui/border-beam";
export { DotPattern } from "./components/magicui/dot-pattern";
export { FlickeringGrid } from "./components/magicui/flickering-grid";
export { HeroVideoDialog } from "./components/magicui/hero-video";
export { Marquee } from "./components/magicui/marquee";
export { Ripple } from "./components/magicui/ripple";
