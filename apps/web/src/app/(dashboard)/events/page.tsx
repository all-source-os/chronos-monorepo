"use client";

import { useState, useEffect } from "react";
import { useSearchParams, useRouter } from "next/navigation";
import { BlurFade, Button, Input, Card, CardContent } from "@allsource/ui";
import { Search, Filter, Download, Plus, X, Info, RefreshCw } from "lucide-react";
import { EventTimeline } from "@/components/events/event-timeline";
import { EventList } from "@/components/events/event-list";
import { LiveEventFeed } from "@/components/events/live-event-feed";
import { EventDetailDrawer } from "@/components/events/event-detail-drawer";
import { useEvents } from "@/hooks/use-events";
import type { Event } from "@/lib/api/client";

export default function EventsPage() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const [search, setSearch] = useState("");
  const [entityFilter, setEntityFilter] = useState(searchParams.get("entity") || "");
  const [typeFilter, setTypeFilter] = useState("");
  const [showFilters, setShowFilters] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState<Event | null>(null);
  const [showDrawer, setShowDrawer] = useState(false);

  const { events, total, isLoading, error, refresh } = useEvents({
    entity_id: entityFilter || undefined,
    event_type: typeFilter || undefined,
    limit: 50,
  });

  // Track if we're showing demo data
  const hasRealEvents = events.length > 0;
  const showingDemoData = !hasRealEvents && !isLoading;

  // Demo events for timeline if no real events
  const demoEvents: Event[] = hasRealEvents ? events : [
    {
      id: "demo-1",
      entity_id: "user-123",
      event_type: "user.signed_up",
      payload: { email: "demo@example.com" },
      timestamp: new Date(Date.now() - 1000 * 60 * 60 * 24).toISOString(),
      version: 1,
    },
    {
      id: "demo-2",
      entity_id: "order-456",
      event_type: "order.placed",
      payload: { total: 99.99 },
      timestamp: new Date(Date.now() - 1000 * 60 * 60 * 12).toISOString(),
      version: 1,
    },
    {
      id: "demo-3",
      entity_id: "payment-789",
      event_type: "payment.completed",
      payload: { method: "card" },
      timestamp: new Date(Date.now() - 1000 * 60 * 60 * 6).toISOString(),
      version: 1,
    },
    {
      id: "demo-4",
      entity_id: "user-123",
      event_type: "user.profile_updated",
      payload: { field: "name" },
      timestamp: new Date(Date.now() - 1000 * 60 * 60 * 2).toISOString(),
      version: 2,
    },
    {
      id: "demo-5",
      entity_id: "inventory-abc",
      event_type: "inventory.updated",
      payload: { sku: "WIDGET-1", quantity: 100 },
      timestamp: new Date(Date.now() - 1000 * 60 * 30).toISOString(),
      version: 1,
    },
  ];

  // Filter events by search
  const filteredEvents = demoEvents.filter((event) =>
    search
      ? event.event_type.toLowerCase().includes(search.toLowerCase()) ||
        event.entity_id.toLowerCase().includes(search.toLowerCase())
      : true
  );

  const handleEventClick = (event: Event) => {
    setSelectedEvent(event);
    setShowDrawer(true);
  };

  const handleViewEntity = (entityId: string) => {
    setShowDrawer(false);
    setEntityFilter(entityId);
    router.push(`/dashboard/events?entity=${entityId}`);
  };

  const clearFilters = () => {
    setSearch("");
    setEntityFilter("");
    setTypeFilter("");
    router.push("/dashboard/events");
  };

  const hasFilters = search || entityFilter || typeFilter;

  const exportEvents = () => {
    const dataStr = JSON.stringify(filteredEvents, null, 2);
    const blob = new Blob([dataStr], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = `events-${new Date().toISOString().slice(0, 10)}.json`;
    document.body.appendChild(link);
    link.click();
    document.body.removeChild(link);
    URL.revokeObjectURL(url);
  };

  return (
    <div className="space-y-6">
      {/* Header */}
      <BlurFade delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">
              Event Explorer
            </h1>
            <p className="mt-1 text-muted-foreground">
              Browse, search, and analyze your event streams
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={exportEvents}>
              <Download className="mr-1.5 h-4 w-4" />
              Export
            </Button>
            <Button size="sm">
              <Plus className="mr-1.5 h-4 w-4" />
              Create Event
            </Button>
          </div>
        </div>
      </BlurFade>

      {/* Demo Data Notice */}
      {showingDemoData && (
        <BlurFade delay={0.15} inView>
          <div className="flex items-center justify-between rounded-lg border border-primary/20 bg-primary/5 px-4 py-3">
            <div className="flex items-center gap-2 text-sm">
              <Info className="h-4 w-4 text-primary" />
              <span>
                Showing sample events. Create your first event to see real data here.
              </span>
            </div>
            <Button variant="ghost" size="sm" onClick={refresh}>
              <RefreshCw className="mr-1 h-3 w-3" />
              Refresh
            </Button>
          </div>
        </BlurFade>
      )}

      {/* Search and Filters */}
      <BlurFade delay={0.2} inView>
        <Card>
          <CardContent className="p-4">
            <div className="flex flex-col gap-4 sm:flex-row">
              {/* Search */}
              <div className="relative flex-1">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder="Search events by type or entity..."
                  value={search}
                  onChange={(e) => setSearch(e.target.value)}
                  className="pl-9"
                />
              </div>

              {/* Filter toggle */}
              <Button
                variant="outline"
                onClick={() => setShowFilters(!showFilters)}
                className={showFilters ? "bg-muted" : ""}
              >
                <Filter className="mr-1.5 h-4 w-4" />
                Filters
                {hasFilters && (
                  <span className="ml-1.5 rounded-full bg-primary px-1.5 py-0.5 text-[10px] text-primary-foreground">
                    {[entityFilter, typeFilter].filter(Boolean).length || 1}
                  </span>
                )}
              </Button>

              {hasFilters && (
                <Button variant="ghost" size="icon" onClick={clearFilters}>
                  <X className="h-4 w-4" />
                </Button>
              )}
            </div>

            {/* Expanded filters */}
            {showFilters && (
              <div className="mt-4 grid gap-4 border-t border-border pt-4 sm:grid-cols-3">
                <div>
                  <label className="mb-1.5 block text-sm font-medium">
                    Entity ID
                  </label>
                  <Input
                    placeholder="Filter by entity..."
                    value={entityFilter}
                    onChange={(e) => setEntityFilter(e.target.value)}
                  />
                </div>
                <div>
                  <label className="mb-1.5 block text-sm font-medium">
                    Event Type
                  </label>
                  <Input
                    placeholder="Filter by type..."
                    value={typeFilter}
                    onChange={(e) => setTypeFilter(e.target.value)}
                  />
                </div>
                <div>
                  <label className="mb-1.5 block text-sm font-medium">
                    Date Range
                  </label>
                  <Input type="date" placeholder="Select date..." />
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </BlurFade>

      {/* Timeline */}
      <BlurFade delay={0.3} inView>
        <EventTimeline
          events={filteredEvents}
          onEventClick={handleEventClick}
          selectedEventId={selectedEvent?.id}
        />
      </BlurFade>

      {/* Main content grid */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Event List */}
        <BlurFade delay={0.4} inView className="lg:col-span-2">
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">
                Events
                <span className="ml-2 text-sm font-normal text-muted-foreground">
                  ({filteredEvents.length} results)
                </span>
              </h2>
            </div>
            <EventList
              events={filteredEvents}
              onEventClick={handleEventClick}
              selectedEventId={selectedEvent?.id}
              isLoading={isLoading}
            />
          </div>
        </BlurFade>

        {/* Live Feed */}
        <BlurFade delay={0.5} inView>
          <LiveEventFeed onEventClick={handleEventClick} />
        </BlurFade>
      </div>

      {/* Event Detail Drawer */}
      <EventDetailDrawer
        event={selectedEvent}
        open={showDrawer}
        onClose={() => setShowDrawer(false)}
        onViewEntity={handleViewEntity}
      />
    </div>
  );
}
