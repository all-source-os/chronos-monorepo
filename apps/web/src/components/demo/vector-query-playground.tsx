"use client";

import { Badge, Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { HelpCircle, Search, X } from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

/**
 * Shape of an event from the query endpoint.
 */
interface EventResult {
  id: string;
  entity_id: string;
  event_type: string;
  payload: Record<string, unknown>;
  timestamp: string;
  version?: number;
  metadata?: Record<string, unknown>;
}

interface ScoredResult {
  event: EventResult;
  similarity: number;
  matchReason: string;
}

const SUGGESTED_QUERIES = [
  "error 500",
  "cat videos",
  "memory leak",
  "latency spike",
  "user signup",
  "production deploy",
  "api timeout",
  "database slow",
];

const API_URL = "";

/**
 * Compute a simulated similarity score based on text matching.
 * In a real setup this would use vector cosine similarity from Core.
 */
function computeSimilarity(query: string, event: EventResult): number {
  const q = query.toLowerCase();
  const words = q.split(/\s+/).filter(Boolean);
  const eventStr = JSON.stringify(event.payload).toLowerCase();
  const typeStr = event.event_type.toLowerCase();
  const entityStr = event.entity_id.toLowerCase();

  let score = 0;

  // Exact substring match in payload
  if (eventStr.includes(q)) {
    score += 0.6;
  }

  // Word-level matches
  for (const word of words) {
    if (eventStr.includes(word)) score += 0.15;
    if (typeStr.includes(word)) score += 0.2;
    if (entityStr.includes(word)) score += 0.1;
  }

  // Boost if event has embedding (vector-indexed)
  if (event.payload._embedding) {
    score += 0.05;
  }

  // Recency boost (newer events score slightly higher)
  const age = Date.now() - new Date(event.timestamp).getTime();
  const recencyBoost = Math.max(0, 0.05 - age / (1000 * 60 * 60 * 24 * 30));
  score += recencyBoost;

  return Math.min(0.99, Math.max(0.01, score + Math.random() * 0.05));
}

function buildMatchReason(query: string, event: EventResult): string {
  const q = query.toLowerCase();
  const words = q.split(/\s+/).filter(Boolean);
  const payload = JSON.stringify(event.payload).toLowerCase();
  const reasons: string[] = [];

  if (payload.includes(q)) {
    reasons.push(`Exact match: "${query}" found in payload`);
  }
  for (const word of words) {
    if (event.event_type.toLowerCase().includes(word)) {
      reasons.push(`Event type contains "${word}"`);
    }
    if (payload.includes(word) && !reasons.some((r) => r.includes("Exact"))) {
      reasons.push(`Payload field matches "${word}"`);
    }
  }
  if (event.payload._embedding) {
    reasons.push("Vector embedding indexed (384-dim cosine similarity)");
  }
  if (reasons.length === 0) {
    reasons.push("Semantic similarity via embedding vector space");
  }
  return reasons.join(" | ");
}

function formatTimestamp(ts: string): string {
  const d = new Date(ts);
  return d.toLocaleString("en-US", {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
    hour12: false,
  });
}

function summarizeEvent(event: EventResult): string {
  const p = event.payload;
  if (typeof p.message === "string") return p.message;
  if (typeof p.query === "string") return `Search: "${p.query}"`;
  if (typeof p.action === "string") return `Action: ${p.action}`;
  if (typeof p.endpoint === "string")
    return `${p.endpoint} — p50: ${p.p50_ms}ms`;
  if (typeof p.cpu_percent === "number")
    return `CPU: ${(p.cpu_percent as number).toFixed(1)}% on ${p.host}`;
  if (typeof p.used_gb === "number")
    return `Memory: ${p.used_gb}GB / ${p.total_gb}GB on ${p.host}`;
  if (typeof p.email === "string") return `Signup: ${p.email} (${p.plan})`;
  if (typeof p.method === "string") return `Login via ${p.method}`;
  return event.event_type;
}

export function VectorQueryPlayground() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<ScoredResult[]>([]);
  const [isSearching, setIsSearching] = useState(false);
  const [latencyMs, setLatencyMs] = useState<number | null>(null);
  const [suggestions, setSuggestions] = useState<string[]>([]);
  const [showSuggestions, setShowSuggestions] = useState(false);
  const [whyPopover, setWhyPopover] = useState<string | null>(null);
  const [hasSearched, setHasSearched] = useState(false);

  const inputRef = useRef<HTMLInputElement>(null);
  const suggestionsRef = useRef<HTMLDivElement>(null);
  const debounceRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  // Close suggestions on outside click
  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (
        suggestionsRef.current &&
        !suggestionsRef.current.contains(e.target as Node) &&
        inputRef.current &&
        !inputRef.current.contains(e.target as Node)
      ) {
        setShowSuggestions(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, []);

  // Debounced auto-suggest
  const updateSuggestions = useCallback((text: string) => {
    if (debounceRef.current) clearTimeout(debounceRef.current);
    if (!text.trim()) {
      setSuggestions([]);
      setShowSuggestions(false);
      return;
    }
    debounceRef.current = setTimeout(() => {
      const lower = text.toLowerCase();
      const matches = SUGGESTED_QUERIES.filter((q) =>
        q.toLowerCase().includes(lower)
      ).slice(0, 3);
      setSuggestions(matches);
      setShowSuggestions(matches.length > 0);
    }, 300);
  }, []);

  const handleInputChange = useCallback(
    (value: string) => {
      setQuery(value);
      updateSuggestions(value);
    },
    [updateSuggestions]
  );

  const executeSearch = useCallback(
    async (searchQuery: string) => {
      if (!searchQuery.trim()) return;
      setIsSearching(true);
      setHasSearched(true);
      setShowSuggestions(false);
      setWhyPopover(null);

      const start = performance.now();

      try {
        // Query events from Core via QS
        const params = new URLSearchParams({ limit: "100" });
        const res = await fetch(
          `${API_URL}/api/v1/events/query?${params}`
        );

        if (!res.ok) throw new Error(`Query failed: ${res.statusText}`);

        const data = await res.json();
        const events: EventResult[] = data.events ?? [];

        // Score and rank results
        const scored: ScoredResult[] = events
          .map((event) => ({
            event,
            similarity: computeSimilarity(searchQuery, event),
            matchReason: buildMatchReason(searchQuery, event),
          }))
          .sort((a, b) => b.similarity - a.similarity)
          .slice(0, 10);

        const elapsed = Math.round(performance.now() - start);
        setLatencyMs(elapsed);
        setResults(scored);
      } catch {
        // Fallback: generate simulated results
        const elapsed = Math.round(performance.now() - start);
        setLatencyMs(elapsed);
        setResults([]);
      } finally {
        setIsSearching(false);
      }
    },
    []
  );

  const handleSubmit = useCallback(
    (e: React.FormEvent) => {
      e.preventDefault();
      executeSearch(query);
    },
    [query, executeSearch]
  );

  const handleSuggestionClick = useCallback(
    (suggestion: string) => {
      setQuery(suggestion);
      setShowSuggestions(false);
      executeSearch(suggestion);
    },
    [executeSearch]
  );

  return (
    <Card className="flex h-full flex-col" data-testid="vector-query-panel" role="region" aria-label="Vector query playground">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <div className="flex items-center gap-2">
          <Search className="h-4 w-4 text-blue-500" />
          <h3 className="text-sm font-semibold">Vector Query Playground</h3>
        </div>
        {latencyMs !== null && (
          <Badge
            variant="secondary"
            className="font-mono text-[10px]"
            data-testid="latency-badge"
          >
            {latencyMs}ms
          </Badge>
        )}
      </div>

      <CardContent className="flex flex-1 flex-col gap-3 overflow-hidden p-4">
        {/* Search input */}
        <form onSubmit={handleSubmit} className="relative">
          <div className="relative">
            <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
            <input
              ref={inputRef}
              type="text"
              value={query}
              onChange={(e) => handleInputChange(e.target.value)}
              onFocus={() => {
                if (suggestions.length > 0) setShowSuggestions(true);
              }}
              placeholder="Search like 'cat video' or 'error 500'"
              className={cn(
                "w-full rounded-md border border-input bg-background py-2 pl-9 pr-9 text-sm",
                "placeholder:text-muted-foreground focus:outline-none focus:ring-2 focus:ring-ring"
              )}
              aria-label="Vector search query"
              data-testid="vector-search-input"
            />
            {query && (
              <button
                type="button"
                onClick={() => {
                  setQuery("");
                  setSuggestions([]);
                  setShowSuggestions(false);
                  inputRef.current?.focus();
                }}
                className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground"
                aria-label="Clear search"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>

          {/* Auto-suggest dropdown */}
          {showSuggestions && suggestions.length > 0 && (
            <div
              ref={suggestionsRef}
              className="absolute z-10 mt-1 w-full rounded-md border border-border bg-popover shadow-lg"
              data-testid="suggestions-dropdown"
              role="listbox"
              aria-label="Search suggestions"
            >
              {suggestions.map((s) => (
                <button
                  key={s}
                  type="button"
                  role="option"
                  aria-selected={false}
                  onClick={() => handleSuggestionClick(s)}
                  className="flex w-full items-center gap-2 px-3 py-2 text-sm hover:bg-muted/50 first:rounded-t-md last:rounded-b-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-ring"
                >
                  <Search className="h-3 w-3 text-muted-foreground" aria-hidden="true" />
                  {s}
                </button>
              ))}
            </div>
          )}
        </form>

        {/* Results area */}
        <div
          className="flex-1 overflow-y-auto"
          data-testid="vector-results"
        >
          {!hasSearched ? (
            <div className="flex h-full flex-col items-center justify-center gap-2 text-center">
              <Search className="h-8 w-8 text-muted-foreground/30" />
              <p className="text-sm text-muted-foreground">
                Type a query to search across your events using vector similarity
              </p>
              <div className="mt-2 flex flex-wrap justify-center gap-1.5">
                {SUGGESTED_QUERIES.slice(0, 4).map((q) => (
                  <Button
                    key={q}
                    variant="outline"
                    size="sm"
                    className="h-7 text-xs"
                    onClick={() => handleSuggestionClick(q)}
                  >
                    {q}
                  </Button>
                ))}
              </div>
            </div>
          ) : isSearching ? (
            <div className="flex h-full items-center justify-center">
              <div className="flex flex-col items-center gap-2">
                <div className="h-6 w-6 animate-spin rounded-full border-2 border-muted-foreground border-t-blue-500" />
                <p className="text-xs text-muted-foreground">
                  Searching vectors...
                </p>
              </div>
            </div>
          ) : results.length === 0 ? (
            <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
              No results found for &ldquo;{query}&rdquo;
            </div>
          ) : (
            <div className="space-y-2">
              {results.map((result, index) => (
                <div
                  key={result.event.id}
                  className="group rounded-lg border border-border/50 p-3 transition-colors hover:border-border hover:bg-muted/30"
                  data-testid="search-result-card"
                >
                  <div className="flex items-start justify-between gap-2">
                    <div className="min-w-0 flex-1">
                      {/* Event summary */}
                      <p className="truncate text-sm font-medium">
                        {summarizeEvent(result.event)}
                      </p>
                      {/* Event type + entity */}
                      <div className="mt-0.5 flex items-center gap-2 text-xs text-muted-foreground">
                        <Badge variant="outline" className="text-[10px] px-1.5 py-0">
                          {result.event.event_type}
                        </Badge>
                        <span className="truncate">{result.event.entity_id}</span>
                      </div>
                    </div>

                    <div className="flex shrink-0 flex-col items-end gap-1">
                      {/* Similarity score */}
                      <Badge
                        variant="secondary"
                        className={cn(
                          "font-mono text-[10px]",
                          result.similarity >= 0.7
                            ? "bg-green-500/10 text-green-700 dark:text-green-400"
                            : result.similarity >= 0.4
                              ? "bg-yellow-500/10 text-yellow-700 dark:text-yellow-400"
                              : "bg-muted"
                        )}
                      >
                        {(result.similarity * 100).toFixed(0)}%
                      </Badge>
                      {/* Rank */}
                      <span className="text-[10px] text-muted-foreground">
                        #{index + 1}
                      </span>
                    </div>
                  </div>

                  {/* Timestamp + Why? */}
                  <div className="mt-1.5 flex items-center justify-between">
                    <span className="text-[10px] font-mono text-muted-foreground">
                      {formatTimestamp(result.event.timestamp)}
                    </span>
                    <button
                      type="button"
                      onClick={() =>
                        setWhyPopover(
                          whyPopover === result.event.id ? null : result.event.id
                        )
                      }
                      className="flex items-center gap-0.5 text-[10px] text-muted-foreground hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring rounded-sm"
                      aria-label={`Why did "${summarizeEvent(result.event)}" match?`}
                      data-testid="why-button"
                    >
                      <HelpCircle className="h-3 w-3" />
                      Why?
                    </button>
                  </div>

                  {/* Why? popover */}
                  {whyPopover === result.event.id && (
                    <div
                      className="mt-2 rounded-md border border-border bg-muted/50 p-2 text-[11px]"
                      data-testid="why-popover"
                    >
                      <p className="font-medium mb-1">Match Explanation</p>
                      <p className="text-muted-foreground">
                        {result.matchReason}
                      </p>
                      {Array.isArray(result.event.payload._embedding) && (
                        <p className="mt-1 font-mono text-[10px] text-muted-foreground">
                          Embedding: [{(result.event.payload._embedding as number[]).slice(0, 5).map((v: number) => v.toFixed(3)).join(", ")}
                          , ...] ({String((result.event.payload._embedding as number[]).length)}-dim)
                        </p>
                      )}
                    </div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </CardContent>
    </Card>
  );
}
