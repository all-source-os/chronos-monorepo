#!/usr/bin/env bash
# mammoth — durable agent memory installer (macOS / Linux / WSL).
#
# Installs the allsource-prime MCP server and wires it into the agent(s) you
# have. Memory is local-only by default: a .prime/ data dir on your machine,
# durable (WAL + Parquet), no account.
#
#   caveman make few token. mammoth never forget token.
#
# One-line install (auto-detect agents in the current project / home):
#   curl -fsSL https://raw.githubusercontent.com/all-source-os/chronos/main/plugin/mammoth/install.sh | bash
#
# Flags:
#   --agent <name>   Wire a specific agent: claude | cursor | cline | windsurf | gemini
#                    (repeatable). Default: auto-detect.
#   --data-dir <p>   Memory data dir. Default: ./.prime (project-scoped).
#   --global         Use ~/.prime/memory instead of ./.prime.
#   --no-auto-inject Don't pass --auto-inject (skip the pre-message index).
#   --print          Print the MCP stanza and exit; write nothing.
#   -h, --help       This help.
#
# Idempotent: re-running updates the same config blocks and preserves other
# MCP servers already present. Safe to pipe from curl repeatedly.

set -euo pipefail

REPO="all-source-os/chronos"
AGENTS=()
DATA_DIR=""
AUTO_INJECT=1
PRINT_ONLY=0
USE_GLOBAL=0

log()  { printf 'mammoth: %s\n' "$*" >&2; }
die()  { printf 'mammoth: error: %s\n' "$*" >&2; exit 1; }
usage() {
  cat <<'USAGE'
mammoth — durable agent memory installer.

Installs allsource-prime and wires it into your agent(s). Local-only by default.

  curl -fsSL https://raw.githubusercontent.com/all-source-os/chronos/main/plugin/mammoth/install.sh | bash

Flags:
  --agent <name>    claude | cursor | cline | windsurf | gemini (repeatable). Default: auto-detect.
  --data-dir <p>    Memory data dir. Default: ./.prime (project-scoped).
  --global          Use ~/.prime/memory instead of ./.prime.
  --no-auto-inject  Skip the --auto-inject pre-message index.
  --print           Print the MCP stanza and exit; write nothing.
  -h, --help        This help.
USAGE
}

while [ $# -gt 0 ]; do
  case "$1" in
    --agent)          AGENTS+=("$2"); shift 2 ;;
    --data-dir)       DATA_DIR="$2"; shift 2 ;;
    --global)         USE_GLOBAL=1; shift ;;
    --no-auto-inject) AUTO_INJECT=0; shift ;;
    --print)          PRINT_ONLY=1; shift ;;
    -h|--help)        usage; exit 0 ;;
    *)                die "unknown flag: $1 (try --help)" ;;
  esac
done

# --- 1. Ensure the binary -------------------------------------------------
ensure_prime() {
  if command -v allsource-prime >/dev/null 2>&1; then
    log "allsource-prime present: $(command -v allsource-prime)"
    return
  fi
  command -v cargo >/dev/null 2>&1 || die \
    "allsource-prime not found and cargo is missing. Install Rust (https://rustup.rs) or the binary, then re-run."
  log "installing allsource-prime via cargo (needs >= 0.21.3)…"
  cargo install allsource-prime
}

# --- 2. Resolve data dir --------------------------------------------------
resolve_data_dir() {
  if [ -n "$DATA_DIR" ]; then return; fi
  if [ "$USE_GLOBAL" -eq 1 ]; then
    DATA_DIR="$HOME/.prime/memory"
  else
    DATA_DIR="$PWD/.prime"
  fi
}

# --- 3. Build the MCP stanza ---------------------------------------------
# Emits a JSON object with a single "prime" server, ready to merge.
stanza() {
  local args="\"--data-dir\", \"$DATA_DIR\""
  if [ "$AUTO_INJECT" -eq 1 ]; then
    args="$args, \"--auto-inject\", \"--auto-inject-max-tokens\", \"1000\""
  fi
  cat <<EOF
{
  "mcpServers": {
    "prime": {
      "command": "allsource-prime",
      "args": [$args]
    }
  }
}
EOF
}

# --- 4. Merge stanza into an agent config file ---------------------------
# Uses python3 if available for a real JSON merge (preserves other servers);
# otherwise writes the file only if absent, and warns on conflict.
merge_into() {
  local target="$1" label="$2"
  mkdir -p "$(dirname "$target")"
  if command -v python3 >/dev/null 2>&1; then
    DATA_DIR="$DATA_DIR" AUTO_INJECT="$AUTO_INJECT" TARGET="$target" python3 - <<'PY'
import json, os, sys
target = os.environ["TARGET"]
args = ["--data-dir", os.environ["DATA_DIR"]]
if os.environ["AUTO_INJECT"] == "1":
    args += ["--auto-inject", "--auto-inject-max-tokens", "1000"]
prime = {"command": "allsource-prime", "args": args}
doc = {}
if os.path.exists(target):
    try:
        with open(target) as f:
            doc = json.load(f) or {}
    except (json.JSONDecodeError, OSError):
        print(f"  (existing {target} unreadable — leaving it; add the stanza by hand)", file=sys.stderr)
        sys.exit(0)
doc.setdefault("mcpServers", {})["prime"] = prime  # idempotent upsert; other servers preserved
with open(target, "w") as f:
    json.dump(doc, f, indent=2)
    f.write("\n")
print(f"  wrote prime MCP server -> {target}", file=sys.stderr)
PY
  else
    if [ -e "$target" ]; then
      log "  $target exists and python3 is missing — add the stanza by hand:"; stanza >&2
    else
      stanza > "$target"; log "  wrote $target"
    fi
  fi
  log "  ($label) reload the agent to pick up the prime server"
}

# --- 5. Agent detection + wiring -----------------------------------------
wire_claude() {
  # Claude Code: project-scoped .mcp.json at repo root.
  merge_into "$PWD/.mcp.json" "Claude Code"
}
wire_cursor()   { merge_into "$PWD/.cursor/mcp.json" "Cursor"; }
wire_cline()    { log "Cline: add the stanza via Cline's MCP settings UI (cline_mcp_settings.json):"; stanza >&2; }
wire_windsurf() { merge_into "$HOME/.codeium/windsurf/mcp_config.json" "Windsurf"; }
wire_gemini()   { merge_into "$HOME/.gemini/settings.json" "Gemini CLI"; }

detect_agents() {
  local found=()
  command -v claude  >/dev/null 2>&1 && found+=(claude)
  [ -d "$PWD/.cursor" ] || command -v cursor >/dev/null 2>&1 && found+=(cursor)
  [ -d "$HOME/.codeium/windsurf" ] && found+=(windsurf)
  command -v gemini  >/dev/null 2>&1 && found+=(gemini)
  if [ ${#found[@]} -eq 0 ]; then
    log "no agents auto-detected; defaulting to Claude Code (project .mcp.json)"
    found=(claude)
  fi
  printf '%s\n' "${found[@]}"
}

# --- main -----------------------------------------------------------------
resolve_data_dir

if [ "$PRINT_ONLY" -eq 1 ]; then
  stanza
  exit 0
fi

ensure_prime

if [ ${#AGENTS[@]} -eq 0 ]; then
  while IFS= read -r a; do AGENTS+=("$a"); done < <(detect_agents)
fi

log "wiring memory (data dir: $DATA_DIR) into: ${AGENTS[*]}"
for a in "${AGENTS[@]}"; do
  case "$a" in
    claude)   wire_claude ;;
    cursor)   wire_cursor ;;
    cline)    wire_cline ;;
    windsurf) wire_windsurf ;;
    gemini)   wire_gemini ;;
    *)        log "unknown agent '$a' — skipping (known: claude cursor cline windsurf gemini)" ;;
  esac
done

log "done. Local-only memory, no account. Cross-machine sync is the upgrade:"
log "  add --sync-to <core-url> --api-key <key> to the prime args (free AllSource account)."
log "For the richest Claude Code setup, install the plugin instead:"
log "  /plugin marketplace add $REPO   then   /plugin install mammoth"
