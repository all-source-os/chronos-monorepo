/**
 * Regex helpers for building URL matchers in specs.
 *
 * Several specs built a matcher with `path.replace(/\//g, "\\/")`, which
 * escapes forward slashes and nothing else. Forward slashes do not even need
 * escaping inside a `new RegExp(...)` string — while `.`, `?`, `+`, `(` and `[`
 * DO, and those appear in real routes and query strings. The result was a
 * matcher looser than intended: `.` matched any character, so a test asserting
 * `/dashboard/api-keys` would also pass on `/dashboardXapi-keys`.
 *
 * CodeQL flagged this as js/incomplete-sanitization.
 */

/**
 * Escapes every regular-expression metacharacter in `literal`, so the result
 * matches exactly that text when embedded in a pattern.
 */
export function escapeRegExp(literal: string): string {
  return literal.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Builds a matcher for a URL path that must appear literally.
 *
 * Anchored at the end so `/dashboard` does not silently satisfy a test that
 * meant `/dashboard/events`.
 */
export function urlPathMatcher(path: string): RegExp {
  return new RegExp(`${escapeRegExp(path)}(?:[?#].*)?$`);
}
