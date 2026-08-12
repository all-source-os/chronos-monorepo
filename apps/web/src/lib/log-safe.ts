/**
 * Sanitises untrusted values before they are written to server logs.
 *
 * Log injection (CodeQL js/log-injection): any user-controlled string that
 * reaches a log line can contain CR/LF and forge additional entries. On a
 * feedback endpoint the whole payload is attacker-chosen, so an unescaped title
 * or body can write arbitrary lines into the server log — enough to fake an
 * entry, bury a real one, or break a parser that splits on newlines.
 *
 * Sanitising at the log call site is deliberate: the same text is legitimately
 * allowed to contain newlines when it becomes a GitHub issue body. Only the log
 * line is dangerous, so only the log line is escaped.
 */

/** Long enough for a feedback title or a stack trace line; short enough that a
 * megabyte of junk cannot flood the log. */
const MAX_LEN = 2000;

export function logSafe(value: unknown): string {
  let text = typeof value === "string" ? value : String(value);

  let truncated = false;
  if (text.length > MAX_LEN) {
    text = text.slice(0, MAX_LEN);
    truncated = true;
  }

  // Replace rather than delete: the value's shape stays visible, so a
  // forged-looking entry reads as escaped text instead of silently
  // reassembling into something that looks like a real log line.
  const escaped = text
    .replace(/\r\n|\r|\n/g, "\\n")
    .replace(/\t/g, "\\t")
    // Remaining C0 controls + DEL: terminal escapes, NUL, etc.
    // biome-ignore lint/suspicious/noControlCharactersInRegex: matching control characters is the entire point
    .replace(/[\u0000-\u001F\u007F]/g, "?");

  return truncated ? `${escaped}…(truncated)` : escaped;
}
