import { fetchCatalog } from "@/lib/pricing-catalog";

// Client-facing proxy for the control-plane pricing catalog. The dashboard
// billing page (a client component) can't read CONTROL_PLANE_INTERNAL_URL, so it
// fetches here; the marketing /pricing page fetches `fetchCatalog()` directly in
// its server component. Empty catalog on failure → frontend uses config prices.
export async function GET() {
  const catalog = (await fetchCatalog()) ?? { currency: "USD", tiers: [] };
  return Response.json(catalog, {
    headers: { "Cache-Control": "public, max-age=300, stale-while-revalidate=3600" },
  });
}
