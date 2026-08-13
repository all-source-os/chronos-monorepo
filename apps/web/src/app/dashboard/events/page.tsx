"use client";

import { Button, Card, CardContent, Input } from "@allsource/ui";
import {
  AlertCircle,
  BookOpen,
  Download,
  Eye,
  EyeOff,
  Filter,
  Inbox,
  Plus,
  RefreshCw,
  Search,
  X,
} from "lucide-react";
import dynamic from "next/dynamic";
import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { CreateEventDialog } from "@/components/events/create-event-dialog";
import { EventList } from "@/components/events/event-list";
import { EventTimeline } from "@/components/events/event-timeline";
import { LiveEventFeed } from "@/components/events/live-event-feed";
import { FadeIn } from "@/components/ui/fade-in";
import { useEvents } from "@/hooks/use-events";
import type { Event } from "@/lib/api/client";
import { eventMatchesLocalFilters } from "@/lib/event-filters";
import { PLATFORM_NOISE_PREFIX_PARAM } from "@/lib/event-namespaces";

const EventDetailDrawer = dynamic(
  () =>
    import("@/components/events/event-detail-drawer").then((module) => module.EventDetailDrawer),
  { ssr: false }
);

export default function EventsPage() {
  const router = useRouter();
  const searchParams = useSearchParams();

  const [search, setSearch] = useState("");
  const [entityFilter, setEntityFilter] = useState(searchParams.get("entity") || "");
  const [typeFilter, setTypeFilter] = useState("");
  const [fromDate, setFromDate] = useState("");
  const [showFilters, setShowFilters] = useState(false);
  const [showSystem, setShowSystem] = useState(false);
  const [selectedEvent, setSelectedEvent] = useState<Event | null>(null);
  const [showDrawer, setShowDrawer] = useState(false);
  const [showCreateDialog, setShowCreateDialog] = useState(false);

  const { events, total, isLoading, error, createEvent, refresh } = useEvents({
    entity_id: entityFilter || undefined,
    event_type: typeFilter || undefined,
    // Hide platform-noise namespaces (heartbeats/audit/_system) by default so
    // operational chatter doesn't bury domain events. Skip the exclusion when
    // the user is explicitly filtering by a type, or toggled "show system".
    exclude_event_type_prefix: showSystem || typeFilter ? undefined : PLATFORM_NOISE_PREFIX_PARAM,
    limit: 50,
  });

  const showingEmptyState = events.length === 0 && !isLoading;

  useEffect(() => {
    if (searchParams.get("action") === "create") {
      setShowCreateDialog(true);
    }
  }, [searchParams]);

  // Filter events by search
  const filteredEvents = events.filter((event) =>
    eventMatchesLocalFilters(event, search, fromDate)
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
    setFromDate("");
    router.push("/dashboard/events");
  };

  const setCreateDialogOpen = (open: boolean) => {
    setShowCreateDialog(open);
    if (!open && searchParams.get("action") === "create") {
      router.replace("/dashboard/events", { scroll: false });
    }
  };

  const hasFilters = search || entityFilter || typeFilter || fromDate;

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
      <FadeIn delay={0.1} inView>
        <div className="flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
          <div>
            <h1 className="text-2xl font-bold tracking-tight md:text-3xl">Event Explorer</h1>
            <p className="mt-1 text-muted-foreground">
              Browse, search, and analyze your event streams
            </p>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="outline" size="sm" onClick={exportEvents}>
              <Download className="mr-1.5 h-4 w-4" />
              Export
            </Button>
            <Button size="sm" onClick={() => setShowCreateDialog(true)}>
              <Plus className="mr-1.5 h-4 w-4" />
              Create Event
            </Button>
          </div>
        </div>
      </FadeIn>

      {/* Read failure */}
      {error && (
        <Card className="border-destructive/40 bg-destructive/5">
          <CardContent className="flex flex-col items-start gap-4 p-5 sm:flex-row sm:items-center">
            <AlertCircle className="h-5 w-5 shrink-0 text-destructive" />
            <div className="flex-1">
              <p className="font-medium">Events could not be loaded</p>
              <p className="text-sm text-muted-foreground">{error}</p>
            </div>
            <Button variant="outline" size="sm" onClick={refresh}>
              <RefreshCw className="mr-1.5 h-4 w-4" />
              Try again
            </Button>
          </CardContent>
        </Card>
      )}

      {/* Empty State */}
      {showingEmptyState && !error && (
        <FadeIn delay={0.15} inView>
          <Card className="p-12 text-center">
            <Inbox className="mx-auto mb-4 h-12 w-12 text-muted-foreground/50" />
            <h3 className="text-lg font-medium">No events yet</h3>
            <p className="mx-auto mt-2 max-w-md text-sm text-muted-foreground">
              Start sending events to your store to see them here. Use the API or any of our SDKs to
              ingest your first event.
            </p>
            <div className="mx-auto mt-6 max-w-lg rounded-lg bg-muted p-4 text-left">
              <p className="mb-2 text-xs font-medium text-muted-foreground">
                Quick start with curl:
              </p>
              <pre className="overflow-x-auto text-xs">
                <code>{`curl -X POST $ALLSOURCE_URL/api/v1/events \\
  -H "Authorization: Bearer $API_KEY" \\
  -H "Content-Type: application/json" \\
  -d '{"entity_id":"user-1","event_type":"user.created","payload":{"name":"Alice"}}'`}</code>
              </pre>
            </div>
            <div className="mt-6 flex items-center justify-center gap-3">
              <Button size="sm" onClick={() => setShowCreateDialog(true)}>
                <Plus className="mr-1.5 h-4 w-4" />
                Create first event
              </Button>
              <Button variant="outline" size="sm" asChild>
                <a href="https://docs.all-source.xyz" target="_blank" rel="noopener noreferrer">
                  <BookOpen className="mr-1.5 h-4 w-4" />
                  Read the Docs
                </a>
              </Button>
              <Button size="sm" onClick={refresh}>
                <RefreshCw className="mr-1.5 h-4 w-4" />
                Refresh
              </Button>
            </div>
          </Card>
        </FadeIn>
      )}

      {/* Search and Filters */}
      <FadeIn delay={0.2} inView>
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

              {/* Show/hide platform-noise namespaces (heartbeats, audit, system) */}
              <Button
                variant="outline"
                onClick={() => setShowSystem((s) => !s)}
                className={showSystem ? "bg-muted" : ""}
                title={showSystem ? "Hide system events" : "Show system events (heartbeats, audit)"}
              >
                {showSystem ? (
                  <Eye className="mr-1.5 h-4 w-4" />
                ) : (
                  <EyeOff className="mr-1.5 h-4 w-4" />
                )}
                System
              </Button>

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
                  <label htmlFor="event-entity-filter" className="mb-1.5 block text-sm font-medium">
                    Entity ID
                  </label>
                  <Input
                    id="event-entity-filter"
                    placeholder="Filter by entity..."
                    value={entityFilter}
                    onChange={(e) => setEntityFilter(e.target.value)}
                  />
                </div>
                <div>
                  <label htmlFor="event-type-filter" className="mb-1.5 block text-sm font-medium">
                    Event Type
                  </label>
                  <Input
                    id="event-type-filter"
                    placeholder="Filter by type..."
                    value={typeFilter}
                    onChange={(e) => setTypeFilter(e.target.value)}
                  />
                </div>
                <div>
                  <label htmlFor="event-from-date" className="mb-1.5 block text-sm font-medium">
                    On or after
                  </label>
                  <Input
                    id="event-from-date"
                    type="date"
                    value={fromDate}
                    onChange={(event) => setFromDate(event.target.value)}
                  />
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </FadeIn>

      {/* Timeline */}
      <FadeIn delay={0.3} inView>
        <EventTimeline
          events={filteredEvents}
          onEventClick={handleEventClick}
          selectedEventId={selectedEvent?.id}
        />
      </FadeIn>

      {/* Main content grid */}
      <div className="grid gap-6 lg:grid-cols-3">
        {/* Event List */}
        <FadeIn delay={0.4} inView className="lg:col-span-2">
          <div className="space-y-4">
            <div className="flex items-center justify-between">
              <h2 className="text-lg font-semibold">
                Events
                <span className="ml-2 text-sm font-normal text-muted-foreground">
                  ({filteredEvents.length} shown
                  {total > filteredEvents.length ? ` of ${total}` : ""})
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
        </FadeIn>

        {/* Live Feed */}
        <FadeIn delay={0.5} inView>
          <LiveEventFeed onEventClick={handleEventClick} />
        </FadeIn>
      </div>

      {/* Event Detail Drawer */}
      <EventDetailDrawer
        event={selectedEvent}
        open={showDrawer}
        onClose={() => setShowDrawer(false)}
        onViewEntity={handleViewEntity}
      />

      <CreateEventDialog
        open={showCreateDialog}
        onOpenChange={setCreateDialogOpen}
        onCreate={createEvent}
      />
    </div>
  );
}
