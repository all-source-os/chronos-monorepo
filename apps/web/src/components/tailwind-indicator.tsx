export function TailwindIndicator() {
  // Don't show in production
  if (process.env.NODE_ENV === "production") return null;
  return (
    <div className="fixed bottom-12 left-3 z-50 flex h-8 w-8 items-center justify-center rounded-full bg-background/80 backdrop-blur-sm border border-border shadow-lg p-1">
      {/* eslint-disable-next-line @next/next/no-img-element */}
      <img src="/logo.png" alt="all.source" className="h-6 w-6 object-contain" />
    </div>
  );
}
