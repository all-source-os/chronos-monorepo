#!/usr/bin/env bash
# AllSource Prime — Claude Desktop installer
#
# Installs the `allsource-prime` MCP server and wires it into Claude Desktop's
# config so it loads on next launch. Designed to be one-shot via:
#
#   curl -fsSL https://www.all-source.xyz/install.sh | ALLSOURCE_API_KEY=ask_xxx sh
#
# Steps:
#   1. Detect OS (macOS / Linux first-class; Windows users get manual path)
#   2. Read API key from ALLSOURCE_API_KEY env var; fall back to interactive
#      prompt only if running on a TTY (curl-pipe mode skips the prompt)
#   3. Install the allsource-prime binary
#        — prefers a prebuilt download from GitHub Releases when available
#        — falls back to `cargo install allsource-prime` otherwise
#   4. Locate Claude Desktop config; merge a `prime` entry into mcpServers
#      using python3 (avoids a hard jq dependency)
#   5. Print "Restart Claude Desktop" next-step
#
# Re-running is idempotent: the prime entry is overwritten in place, other
# mcpServers entries are preserved.

set -euo pipefail

readonly REMOTE_SYNC_URL="${ALLSOURCE_REMOTE_URL:-https://api.all-source.xyz}"
readonly CRATES_PACKAGE="allsource-prime"
readonly DATA_DIR_DEFAULT="${ALLSOURCE_PRIME_DATA_DIR:-${HOME}/.prime/memory}"

# Prebuilt-binary release pipeline (task #9) — flip this on once
# allsource-prime-vX.Y.Z tags publish darwin-arm64 / darwin-x64 /
# linux-x64 binaries under github.com/all-source-os/all-source/releases.
readonly USE_PREBUILT_BINARIES="${ALLSOURCE_USE_PREBUILT:-0}"

# ─── Colors (skip if not a tty or NO_COLOR set) ──────────────────────────────
if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  c_bold=$'\033[1m'
  c_dim=$'\033[2m'
  c_red=$'\033[31m'
  c_grn=$'\033[32m'
  c_ylw=$'\033[33m'
  c_off=$'\033[0m'
else
  c_bold=""; c_dim=""; c_red=""; c_grn=""; c_ylw=""; c_off=""
fi

info() { printf '%s→%s %s\n' "$c_grn" "$c_off" "$*"; }
warn() { printf '%s!%s %s\n' "$c_ylw" "$c_off" "$*" >&2; }
err()  { printf '%s✗%s %s\n' "$c_red" "$c_off" "$*" >&2; }
die()  { err "$*"; exit 1; }

# ─── Step 1: OS detection ────────────────────────────────────────────────────
detect_platform() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"
  case "$os" in
    Darwin) platform_os="macos" ;;
    Linux)  platform_os="linux" ;;
    *)
      die "Unsupported OS: $os. Windows users: follow the manual setup at https://www.all-source.xyz/connect"
      ;;
  esac
  case "$arch" in
    x86_64|amd64) platform_arch="x64" ;;
    arm64|aarch64) platform_arch="arm64" ;;
    *) die "Unsupported architecture: $arch" ;;
  esac
  info "Detected ${c_bold}${platform_os}-${platform_arch}${c_off}"
}

# ─── Step 2: Resolve API key ─────────────────────────────────────────────────
resolve_api_key() {
  if [ -n "${ALLSOURCE_API_KEY:-}" ]; then
    api_key="$ALLSOURCE_API_KEY"
    info "Using API key from ALLSOURCE_API_KEY"
    return
  fi
  if [ ! -t 0 ]; then
    err "No ALLSOURCE_API_KEY provided and stdin is not a TTY (likely curl-piped)."
    err "Re-run with the key:"
    err "  curl -fsSL https://www.all-source.xyz/install.sh | ALLSOURCE_API_KEY=ask_xxx sh"
    err "Or generate one at https://www.all-source.xyz/connect"
    exit 1
  fi
  printf '%sAPI key%s (from https://www.all-source.xyz/connect): ' "$c_bold" "$c_off"
  read -r api_key
  [ -n "$api_key" ] || die "API key required"
}

# ─── Step 3: Install the binary ──────────────────────────────────────────────
install_binary() {
  if command -v allsource-prime >/dev/null 2>&1; then
    info "allsource-prime already installed at $(command -v allsource-prime)"
    return
  fi

  if [ "$USE_PREBUILT_BINARIES" = "1" ]; then
    install_prebuilt && return 0
    warn "Prebuilt download failed; falling back to cargo install."
  fi

  if command -v cargo >/dev/null 2>&1; then
    info "Installing via ${c_bold}cargo install $CRATES_PACKAGE${c_off} (a few minutes)…"
    cargo install "$CRATES_PACKAGE"
    return
  fi

  err "Neither a prebuilt binary nor cargo is available."
  err "Install Rust first (https://rustup.rs), then re-run this installer."
  exit 1
}

install_prebuilt() {
  # Placeholder — wired up once GH Releases publishes platform binaries.
  # Expected layout once live:
  #   https://github.com/all-source-os/all-source/releases/download/
  #     allsource-prime-vX.Y.Z/allsource-prime-${platform_os}-${platform_arch}
  warn "Prebuilt-binary install path not yet enabled. (Tracked as task #9.)"
  return 1
}

# ─── Step 4: Merge config into Claude Desktop ────────────────────────────────
claude_desktop_config_path() {
  case "$platform_os" in
    macos) printf '%s/Library/Application Support/Claude/claude_desktop_config.json' "$HOME" ;;
    linux) printf '%s/.config/Claude/claude_desktop_config.json' "$HOME" ;;
  esac
}

write_config() {
  local config_path config_dir
  config_path="$(claude_desktop_config_path)"
  config_dir="$(dirname "$config_path")"
  mkdir -p "$config_dir"

  if ! command -v python3 >/dev/null 2>&1; then
    die "python3 is required to merge the Claude Desktop config (used to preserve existing mcpServers entries). Install python3 and re-run."
  fi

  info "Merging prime entry into ${c_dim}${config_path}${c_off}"

  ALLSOURCE_CONFIG_PATH="$config_path" \
  ALLSOURCE_API_KEY_VAL="$api_key" \
  ALLSOURCE_DATA_DIR="$DATA_DIR_DEFAULT" \
  ALLSOURCE_REMOTE="$REMOTE_SYNC_URL" \
  python3 <<'PYEOF'
import json
import os
import sys

path = os.environ["ALLSOURCE_CONFIG_PATH"]
api_key = os.environ["ALLSOURCE_API_KEY_VAL"]
data_dir = os.environ["ALLSOURCE_DATA_DIR"]
remote = os.environ["ALLSOURCE_REMOTE"]

# Load existing config if present; tolerate empty/missing/malformed
if os.path.exists(path):
    try:
        with open(path, "r", encoding="utf-8") as fh:
            text = fh.read().strip()
        config = json.loads(text) if text else {}
    except json.JSONDecodeError as e:
        print(f"existing config at {path} is not valid JSON: {e}", file=sys.stderr)
        sys.exit(2)
else:
    config = {}

if not isinstance(config, dict):
    print(f"existing config at {path} is not a JSON object", file=sys.stderr)
    sys.exit(2)

mcp = config.setdefault("mcpServers", {})
if not isinstance(mcp, dict):
    print("mcpServers must be a JSON object", file=sys.stderr)
    sys.exit(2)

mcp["prime"] = {
    "command": "allsource-prime",
    "args": [
        "--data-dir", data_dir,
        "--auto-inject",
        "--sync-to", remote,
        "--api-key", api_key,
    ],
}

with open(path, "w", encoding="utf-8") as fh:
    json.dump(config, fh, indent=2)
    fh.write("\n")
PYEOF
}

# ─── Step 5: Next steps ──────────────────────────────────────────────────────
print_next_steps() {
  cat <<EOF

${c_grn}${c_bold}Installed.${c_off}

Next:
  1. ${c_bold}Quit and reopen Claude Desktop${c_off} so it loads the new MCP server.
  2. Ask Claude: ${c_dim}"list the MCP tools you have available"${c_off}.
     You should see ${c_bold}prime_add_node${c_off}, ${c_bold}prime_recall${c_off}, and friends.
  3. Watch your nodes appear at ${c_bold}https://www.all-source.xyz/dashboard/memory${c_off}.

Manage or rotate this key at ${c_bold}https://www.all-source.xyz/dashboard/api-keys${c_off}.
EOF
}

# ─── Main ─────────────────────────────────────────────────────────────────────
main() {
  printf '%sAllSource Prime — Claude Desktop installer%s\n\n' "$c_bold" "$c_off"
  detect_platform
  resolve_api_key
  install_binary
  write_config
  print_next_steps
}

main "$@"
