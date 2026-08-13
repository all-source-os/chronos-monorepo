import { Button, Card, CardContent } from "@allsource/ui";
import { AlertTriangle, RefreshCw } from "lucide-react";

export function LoadError({
  title,
  message,
  onRetry,
}: {
  title: string;
  message?: string;
  onRetry: () => unknown;
}) {
  return (
    <Card className="border-destructive/30 bg-destructive/5" role="alert">
      <CardContent className="flex flex-col gap-4 p-5 sm:flex-row sm:items-center">
        <AlertTriangle className="h-5 w-5 shrink-0 text-destructive" aria-hidden="true" />
        <div className="min-w-0 flex-1">
          <p className="font-medium">{title}</p>
          {message && <p className="mt-1 text-sm text-muted-foreground">{message}</p>}
        </div>
        <Button type="button" variant="outline" size="sm" onClick={() => onRetry()}>
          <RefreshCw className="mr-1.5 h-3.5 w-3.5" aria-hidden="true" />
          Try again
        </Button>
      </CardContent>
    </Card>
  );
}
