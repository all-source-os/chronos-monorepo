"use client";

import { Badge, Input } from "@allsource/ui";
import { Check, Loader2, Search } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import { fetchTenants, planLabel, type Tenant } from "@/lib/tenants-api";

interface TenantSelectProps {
  /** Selected tenant id. */
  value: string;
  /** Called with the chosen tenant's id and name. */
  onChange: (id: string, name: string) => void;
}

/**
 * Searchable tenant picker for the connect dialog. Queries the admin tenants
 * list (server-side search by name/email) as you type — debounced — and resolves
 * to a tenant_id, so an operator never pastes a raw `tnt_…`. Inline dropdown (no
 * Popover portal) to stay simple and focus-safe inside the connect modal.
 */
export function TenantSelect({ value, onChange }: TenantSelectProps) {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<Tenant[]>([]);
  const [loading, setLoading] = useState(false);
  const [open, setOpen] = useState(false);
  const [pickedName, setPickedName] = useState("");
  const boxRef = useRef<HTMLDivElement>(null);

  // Debounced search whenever the query changes while the dropdown is open.
  useEffect(() => {
    if (!open) return;
    let cancelled = false;
    setLoading(true);
    const timer = setTimeout(async () => {
      try {
        const res = await fetchTenants({ search: query.trim() || undefined, per_page: 20 });
        if (!cancelled) setResults(Array.isArray(res.tenants) ? res.tenants : []);
      } catch {
        if (!cancelled) setResults([]);
      } finally {
        if (!cancelled) setLoading(false);
      }
    }, 250);
    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [query, open]);

  // Close the dropdown on an outside click.
  useEffect(() => {
    if (!open) return;
    const onDocClick = (e: MouseEvent) => {
      if (boxRef.current && !boxRef.current.contains(e.target as Node)) setOpen(false);
    };
    document.addEventListener("mousedown", onDocClick);
    return () => document.removeEventListener("mousedown", onDocClick);
  }, [open]);

  const select = (t: Tenant) => {
    onChange(t.id, t.name);
    setPickedName(t.name);
    setQuery(t.name);
    setOpen(false);
  };

  return (
    <div className="relative" ref={boxRef}>
      <div className="relative">
        <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
        <Input
          value={query}
          onChange={(e) => {
            setQuery(e.target.value);
            setPickedName("");
            setOpen(true);
          }}
          onFocus={() => setOpen(true)}
          placeholder="Search tenants by name or email…"
          autoComplete="off"
          className="pl-8"
          data-testid="connect-tenant-search"
        />
      </div>

      {value && pickedName && (
        <p className="mt-1 text-xs text-muted-foreground" data-testid="connect-tenant-selected">
          Selected <span className="font-medium text-foreground">{pickedName}</span>{" "}
          <span className="font-mono">({value})</span>
        </p>
      )}

      {open && (
        <div
          className="absolute z-50 mt-1 max-h-60 w-full overflow-auto rounded-md border bg-popover p-1 shadow-md"
          data-testid="connect-tenant-results"
        >
          {loading ? (
            <div className="flex items-center gap-2 px-2 py-3 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" /> Searching…
            </div>
          ) : results.length === 0 ? (
            <div className="px-2 py-3 text-sm text-muted-foreground">No tenants found.</div>
          ) : (
            results.map((t) => (
              <button
                key={t.id}
                type="button"
                onClick={() => select(t)}
                className="flex w-full items-center justify-between gap-2 rounded-sm px-2 py-1.5 text-left text-sm hover:bg-accent"
                data-testid="connect-tenant-option"
              >
                <span className="flex min-w-0 items-center gap-2">
                  {value === t.id && <Check className="h-3.5 w-3.5 shrink-0" />}
                  <span className="truncate">{t.name}</span>
                  <span className="truncate font-mono text-xs text-muted-foreground">{t.id}</span>
                </span>
                <Badge variant="outline" className="shrink-0 capitalize">
                  {planLabel(t.plan)}
                </Badge>
              </button>
            ))
          )}
        </div>
      )}
    </div>
  );
}
