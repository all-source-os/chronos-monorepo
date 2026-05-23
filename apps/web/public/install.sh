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

# Prebuilt-binary path. The monorepo release pipeline ships:
#   allsource-prime-linux-x86_64.tar.gz   (linux + x64)
#   allsource-prime-macos-aarch64.tar.gz  (macos + arm64)
# Other combinations (linux-arm64, macos-x64) fall back to cargo install.
readonly USE_PREBUILT_BINARIES="${ALLSOURCE_USE_PREBUILT:-1}"
readonly RELEASES_REPO="all-source-os/all-source"
readonly INSTALL_BIN_DIR="${ALLSOURCE_INSTALL_BIN:-${HOME}/.local/bin}"

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
  local artifact tag tarball_url tmp_dir tarball

  # Map detected platform to the artifact name published by release.yml.
  # The Prime release matrix intentionally omits linux-arm64 and
  # macos-x86_64 (ort/fastembed doesn't ship prebuilts for Intel macOS),
  # so those platforms fall back to cargo install.
  case "${platform_os}-${platform_arch}" in
    linux-x64)    artifact="allsource-prime-linux-x86_64" ;;
    macos-arm64)  artifact="allsource-prime-macos-aarch64" ;;
    *)
      warn "No prebuilt binary published for ${platform_os}-${platform_arch} yet; falling back to cargo install."
      return 1
      ;;
  esac

  tag="$(resolve_latest_release_tag)" || {
    warn "Could not resolve latest release tag from GitHub; falling back to cargo install."
    return 1
  }

  tarball_url="https://github.com/${RELEASES_REPO}/releases/download/${tag}/${artifact}.tar.gz"
  tmp_dir="$(mktemp -d)"
  tarball="${tmp_dir}/${artifact}.tar.gz"

  info "Downloading ${c_bold}${artifact}${c_off} from ${c_dim}${tag}${c_off}"
  if ! curl -fsSL --retry 2 -o "$tarball" "$tarball_url"; then
    warn "Download failed: $tarball_url"
    rm -rf "$tmp_dir"
    return 1
  fi

  if ! tar -xzf "$tarball" -C "$tmp_dir"; then
    warn "Failed to extract $tarball"
    rm -rf "$tmp_dir"
    return 1
  fi

  mkdir -p "$INSTALL_BIN_DIR"
  install -m 0755 "${tmp_dir}/${artifact}" "${INSTALL_BIN_DIR}/allsource-prime"
  rm -rf "$tmp_dir"

  # Apple Gatekeeper flags downloaded binaries with com.apple.quarantine,
  # which pops a one-shot dialog on first run. Strip it so the curl-pipe
  # install just works. We never run anything besides our own binary, so
  # this is scoped to the file we just dropped.
  if [ "$platform_os" = "macos" ] && command -v xattr >/dev/null 2>&1; then
    xattr -d com.apple.quarantine "${INSTALL_BIN_DIR}/allsource-prime" 2>/dev/null || true
  fi

  info "Installed to ${c_bold}${INSTALL_BIN_DIR}/allsource-prime${c_off}"

  # Heads-up if the install dir isn't on PATH. Don't try to modify the
  # user's shell rc files — too many shells and configs to mess with.
  case ":${PATH}:" in
    *":${INSTALL_BIN_DIR}:"*) ;;
    *) warn "${INSTALL_BIN_DIR} is not on your PATH. Add this to your shell rc:
       export PATH=\"${INSTALL_BIN_DIR}:\$PATH\"" ;;
  esac

  return 0
}

# Fetch the latest release tag (e.g. v0.21.4) from the GitHub Releases API.
# Falls through cleanly to cargo if the API is unreachable or rate-limited.
resolve_latest_release_tag() {
  local api_url response tag
  api_url="https://api.github.com/repos/${RELEASES_REPO}/releases/latest"
  response="$(curl -fsSL --retry 1 -H 'Accept: application/vnd.github+json' "$api_url" 2>/dev/null)" || return 1
  # Parse JSON with python3 if available (already a hard dep for the
  # config-merge step), else a grep fallback.
  if command -v python3 >/dev/null 2>&1; then
    tag="$(printf '%s' "$response" | python3 -c 'import json,sys; print(json.load(sys.stdin).get("tag_name",""))')"
  else
    tag="$(printf '%s' "$response" | grep -oE '"tag_name":[[:space:]]*"[^"]+"' | head -1 | sed -E 's/.*"([^"]+)"$/\1/')"
  fi
  [ -n "$tag" ] || return 1
  printf '%s' "$tag"
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
