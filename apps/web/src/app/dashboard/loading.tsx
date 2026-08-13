export default function DashboardLoading() {
  return (
    <div className="min-h-screen bg-background p-6" role="status" aria-label="Loading dashboard">
      <div className="mx-auto max-w-7xl animate-pulse space-y-6 pt-16">
        <div className="h-8 w-48 rounded bg-muted" />
        <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-4">
          {["events", "streams", "projections", "latency"].map((metric) => (
            <div key={metric} className="h-32 rounded-xl border border-border bg-card" />
          ))}
        </div>
        <div className="h-72 rounded-xl border border-border bg-card" />
      </div>
    </div>
  );
}
