// Pricing catalog — read from the control-plane `/api/v1/billing/catalog`
// endpoint, which sources live prices from LemonSqueezy (the source of truth for
// what customers are actually charged). Display prices must come from here, not
// from hardcoded numbers, so they can never drift from the real charge.
//
// `siteConfig.pricing` remains the source for tier METADATA (names, features,
// MCP verbs, x402 allowances) and as a price FALLBACK only when the catalog is
// unreachable — never as the authoritative price when the catalog is present.

export type CatalogPrice = {
  cents: number;
  formatted: string; // monthly: per-month (e.g. "$18.99"); annual: total/yr (e.g. "$181.99")
  per_month?: string; // annual only: per-month equivalent (e.g. "$15.17")
};

export type CatalogTier = {
  tier: string;
  monthly?: CatalogPrice;
  annual?: CatalogPrice;
};

export type Catalog = {
  currency: string;
  tiers: CatalogTier[];
};

export type CatalogByTier = Record<string, CatalogTier>;

function controlPlaneUrl(): string {
  return process.env.CONTROL_PLANE_INTERNAL_URL || "http://localhost:3901";
}

/**
 * Server-side fetch of the pricing catalog from the control plane.
 * Returns null on any failure so callers fall back to static config prices.
 * Cached for an hour (prices change rarely; the control plane also caches).
 */
export async function fetchCatalog(): Promise<Catalog | null> {
  try {
    const res = await fetch(`${controlPlaneUrl()}/api/v1/billing/catalog`, {
      next: { revalidate: 3600 },
    });
    if (!res.ok) return null;
    return (await res.json()) as Catalog;
  } catch {
    return null;
  }
}

/** Index a catalog by tier id for O(1) lookup; tolerant of null. */
export function indexByTier(catalog: Catalog | null): CatalogByTier {
  const map: CatalogByTier = {};
  for (const t of catalog?.tiers ?? []) map[t.tier] = t;
  return map;
}
