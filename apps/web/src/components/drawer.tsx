import {
  buttonVariants,
  cn,
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerFooter,
  DrawerHeader,
  DrawerTitle,
  DrawerTrigger,
  Icons,
} from "@allsource/ui";
import { VisuallyHidden } from "@radix-ui/react-visually-hidden";
import Link from "next/link";
import { IoMenuSharp } from "react-icons/io5";
import { siteConfig } from "@/lib/config";
export default function drawerDemo() {
  return (
    <Drawer>
      <DrawerTrigger>
        <IoMenuSharp className="text-2xl" />
      </DrawerTrigger>
      <DrawerContent>
        <DrawerTitle>
          <VisuallyHidden>Navigation</VisuallyHidden>
        </DrawerTitle>
        <DrawerDescription>
          <VisuallyHidden>Navigation</VisuallyHidden>
        </DrawerDescription>
        <DrawerHeader className="px-6">
          <div className="">
            <Link href="/" title="brand-logo" className="relative mr-6 flex items-center space-x-2">
              <Icons.logo className="w-auto h-[40px]" />
              <span className="font-bold text-xl">{siteConfig.name}</span>
            </Link>
          </div>
          <nav>
            <ul className="mt-7 text-left">
              {siteConfig.header.map((item) => (
                <li key={item.label || item.trigger} className="my-3">
                  {item.trigger ? (
                    <span className="font-semibold">{item.trigger}</span>
                  ) : (
                    <Link href={item.href || ""} className="font-semibold">
                      {item.label}
                    </Link>
                  )}
                </li>
              ))}
            </ul>
          </nav>
        </DrawerHeader>
        <DrawerFooter>
          <Link href="/login" className={buttonVariants({ variant: "outline" })}>
            Login
          </Link>
          <Link
            href="/signup"
            className={cn(
              buttonVariants({ variant: "default" }),
              "w-full sm:w-auto text-background flex gap-2"
            )}
          >
            <Icons.logo className="h-6 w-6" />
            Start 14-day trial
          </Link>
        </DrawerFooter>
      </DrawerContent>
    </Drawer>
  );
}
