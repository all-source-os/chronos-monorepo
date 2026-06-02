// Shared color-by-type visual language for the Prime knowledge graph.
// Mirrors the local prime-mcp viewer: same type → same hue across the bubble
// graph, the legend, and the detail list, so the hosted view and the local
// viewer read identically.
//
// Colors are plain hex (not oklch var()) because react-force-graph paints to a
// raw <canvas> and the d3/canvas layer can't resolve CSS custom properties.
// The surrounding chrome (cards, badges, panels) still uses the theme tokens.

const TYPE_COLORS: Record<string, string> = {
  organization: "#6366f1", // indigo
  contact: "#10b981", // emerald
  person: "#10b981", // emerald (alias of contact)
  task: "#f59e0b", // amber
  project: "#8b5cf6", // violet
  document: "#06b6d4", // cyan
  meeting: "#ec4899", // pink
  decision: "#ef4444", // red
  transaction: "#84cc16", // lime
};

const FALLBACK_PALETTE = [
  "#6366f1",
  "#10b981",
  "#f59e0b",
  "#8b5cf6",
  "#06b6d4",
  "#ec4899",
  "#ef4444",
  "#84cc16",
  "#0ea5e9",
  "#f97316",
];

/** Deterministic color for a node type — stable across renders and views. */
export function colorForType(type: string): string {
  const known = TYPE_COLORS[type];
  if (known) return known;
  // Hash the type name to a fallback palette slot so unknown types are stable.
  let hash = 0;
  for (let i = 0; i < type.length; i++) {
    hash = (hash * 31 + type.charCodeAt(i)) | 0;
  }
  return FALLBACK_PALETTE[Math.abs(hash) % FALLBACK_PALETTE.length] ?? "#6366f1";
}

/** Pull a human-readable label out of a node's stored properties. */
export function nodeLabel(properties: Record<string, unknown>, id: string): string {
  for (const key of ["name", "title", "label", "email", "domain"]) {
    const v = properties[key];
    if (typeof v === "string" && v.trim()) return v;
  }
  // Fall back to the uuid tail of `node:<type>:<uuid>`.
  const parts = id.split(":");
  return parts[parts.length - 1] || id;
}
