"use client";

import { Button } from "@allsource/ui";
import { Moon, Sun } from "lucide-react";
import { useTheme } from "next-themes";
import { useEffect, useState } from "react";

export function ThemeToggle() {
  const { setTheme, resolvedTheme } = useTheme();
  const [mounted, setMounted] = useState(false);

  useEffect(() => {
    setMounted(true);
  }, []);

  if (!mounted) {
    return (
      <Button
        variant="ghost"
        size="icon"
        className="fixed bottom-4 right-4 z-50 h-10 w-10 rounded-full border bg-background/80 backdrop-blur-sm shadow-lg"
      >
        <span className="h-5 w-5" />
      </Button>
    );
  }

  const isDark = resolvedTheme === "dark";

  return (
    <Button
      variant="ghost"
      size="icon"
      className="fixed bottom-4 right-4 z-50 h-10 w-10 rounded-full border bg-background/80 backdrop-blur-sm shadow-lg hover:shadow-xl transition-all duration-300 hover:scale-110"
      onClick={() => setTheme(isDark ? "light" : "dark")}
    >
      <Sun className={`h-5 w-5 text-primary transition-all duration-300 ${isDark ? "scale-0 rotate-90 opacity-0" : "scale-100 rotate-0 opacity-100"} absolute`} />
      <Moon className={`h-5 w-5 text-primary transition-all duration-300 ${isDark ? "scale-100 rotate-0 opacity-100" : "scale-0 -rotate-90 opacity-0"} absolute`} />
      <span className="sr-only">Toggle theme</span>
    </Button>
  );
}
