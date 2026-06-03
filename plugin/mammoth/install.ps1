<#
.SYNOPSIS
  mammoth — durable agent memory installer (Windows / PowerShell).

.DESCRIPTION
  Installs the allsource-prime MCP server and wires it into your agent(s).
  Memory is local-only by default: a .prime\ data dir on your machine, durable
  (WAL + Parquet), no account. caveman make few token. mammoth never forget token.

  One-line install:
    irm https://raw.githubusercontent.com/all-source-os/all-source/main/plugin/mammoth/install.ps1 | iex

.PARAMETER Agent
  Wire specific agents: claude, cursor, windsurf, gemini (array). Default: auto-detect.
.PARAMETER DataDir
  Memory data dir. Default: .\.prime (project-scoped).
.PARAMETER Global
  Use $HOME\.prime\memory instead of .\.prime.
.PARAMETER NoAutoInject
  Skip the --auto-inject pre-message index.
.PARAMETER Print
  Print the MCP stanza and exit; write nothing.
#>
[CmdletBinding()]
param(
  [string[]]$Agent = @(),
  [string]$DataDir = "",
  [switch]$Global,
  [switch]$NoAutoInject,
  [switch]$Print
)

$ErrorActionPreference = "Stop"
$Repo = "all-source-os/all-source"
function Log($m) { Write-Host "mammoth: $m" }

# --- data dir ---
if (-not $DataDir) {
  $DataDir = if ($Global) { Join-Path $HOME ".prime\memory" } else { Join-Path (Get-Location) ".prime" }
}

function Get-Stanza {
  $args = @("--data-dir", $DataDir)
  if (-not $NoAutoInject) { $args += @("--auto-inject", "--auto-inject-max-tokens", "1000") }
  $obj = @{ mcpServers = @{ prime = @{ command = "allsource-prime"; args = $args } } }
  return ($obj | ConvertTo-Json -Depth 6)
}

if ($Print) { Get-Stanza; exit 0 }

# --- ensure binary ---
if (-not (Get-Command allsource-prime -ErrorAction SilentlyContinue)) {
  if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    throw "allsource-prime not found and cargo is missing. Install Rust (https://rustup.rs) or the binary, then re-run."
  }
  Log "installing allsource-prime via cargo (needs >= 0.21.6)..."
  cargo install allsource-prime
} else {
  Log "allsource-prime present: $((Get-Command allsource-prime).Source)"
}

# --- merge stanza into a JSON config (idempotent upsert; preserves other servers) ---
function Merge-Into($target, $label) {
  $dir = Split-Path -Parent $target
  if ($dir -and -not (Test-Path $dir)) { New-Item -ItemType Directory -Force -Path $dir | Out-Null }
  $args = @("--data-dir", $DataDir)
  if (-not $NoAutoInject) { $args += @("--auto-inject", "--auto-inject-max-tokens", "1000") }
  $prime = @{ command = "allsource-prime"; args = $args }

  $doc = @{}
  if (Test-Path $target) {
    try { $doc = (Get-Content -Raw $target | ConvertFrom-Json -AsHashtable) }
    catch { Log "  ($target unreadable — leaving it; add the stanza by hand)"; return }
  }
  if (-not $doc.ContainsKey("mcpServers")) { $doc["mcpServers"] = @{} }
  $doc["mcpServers"]["prime"] = $prime
  ($doc | ConvertTo-Json -Depth 6) | Set-Content -Path $target
  Log "  wrote prime MCP server -> $target  ($label) — reload the agent"
}

function Wire($a) {
  switch ($a) {
    "claude"   { Merge-Into (Join-Path (Get-Location) ".mcp.json") "Claude Code" }
    "cursor"   { Merge-Into (Join-Path (Get-Location) ".cursor\mcp.json") "Cursor" }
    "windsurf" { Merge-Into (Join-Path $HOME ".codeium\windsurf\mcp_config.json") "Windsurf" }
    "gemini"   { Merge-Into (Join-Path $HOME ".gemini\settings.json") "Gemini CLI" }
    "cline"    { Log "Cline: add via Cline's MCP settings UI:"; Get-Stanza }
    default    { Log "unknown agent '$a' — skipping (known: claude cursor windsurf gemini cline)" }
  }
}

# --- detect ---
if ($Agent.Count -eq 0) {
  $found = @()
  if (Get-Command claude -ErrorAction SilentlyContinue) { $found += "claude" }
  if ((Test-Path (Join-Path (Get-Location) ".cursor")) -or (Get-Command cursor -ErrorAction SilentlyContinue)) { $found += "cursor" }
  if (Test-Path (Join-Path $HOME ".codeium\windsurf")) { $found += "windsurf" }
  if (Get-Command gemini -ErrorAction SilentlyContinue) { $found += "gemini" }
  if ($found.Count -eq 0) { Log "no agents auto-detected; defaulting to Claude Code"; $found = @("claude") }
  $Agent = $found
}

Log "wiring memory (data dir: $DataDir) into: $($Agent -join ', ')"
foreach ($a in $Agent) { Wire $a }

Log "done. Local-only memory, no account. Cross-machine sync is the upgrade:"
Log "  add --sync-to <core-url> --api-key <key> to the prime args (free AllSource account)."
Log "Richest Claude Code setup: /plugin marketplace add $Repo  then  /plugin install mammoth"
