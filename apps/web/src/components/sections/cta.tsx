import { Icons, Section, buttonVariants, cn } from "@allsource/ui";
import Link from "next/link";

export default function CtaSection() {
  return (
    <Section
      id="cta"
      title="Ready to time-travel your data?"
      subtitle="Start building with AllSource today. Free tier available."
      className="bg-primary/10 rounded-xl py-16"
    >
      <div className="flex flex-col w-full sm:flex-row items-center justify-center space-y-4 sm:space-y-0 sm:space-x-4 pt-4">
        <Link
          href="/signup"
          className={cn(
            buttonVariants({ variant: "default" }),
            "w-full sm:w-auto text-background flex gap-2 px-8"
          )}
        >
          <Icons.logo className="h-5 w-5" />
          Start Building Free
        </Link>
        <Link
          href="https://github.com/allsource/chronos"
          className={cn(
            buttonVariants({ variant: "outline" }),
            "w-full sm:w-auto flex gap-2 px-8"
          )}
        >
          <Icons.github className="h-5 w-5" />
          Star on GitHub
        </Link>
      </div>
    </Section>
  );
}
