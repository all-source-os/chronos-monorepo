#!/usr/bin/env bash
# Scripted demo for asciinema recording of chronis CLI + TUI + web viewer
#
# To record:
#   asciinema rec --cols 110 --rows 32 docs/demos/chronis.cast -c "bash docs/demos/tui-demo.sh"
#
# To render GIF:
#   agg docs/demos/chronis.cast docs/demos/chronis.gif --theme monokai --speed 1.5
#
# To upload:
#   asciinema upload docs/demos/chronis.cast

set -e

DEMO_DIR="/tmp/cn-demo"

typeit() {
  local cmd="$1"
  printf "\033[1;32m>\033[0m "
  for (( i=0; i<${#cmd}; i++ )); do
    printf "%s" "${cmd:$i:1}"
    sleep 0.03
  done
  echo ""
  sleep 0.3
  eval "$cmd"
}

pause() { sleep "${1:-1.2}"; }

header() {
  echo ""
  echo -e "\033[1;36m--- $1 ---\033[0m"
  echo ""
  pause 0.6
}

# ============================================================
clear
echo ""
echo -e "  \033[1;36mchronis\033[0m  event-sourced task management"
echo -e "  \033[0;90mnew: cn tui (ratatui dashboard) + cn serve (web viewer)\033[0m"
echo ""
pause 2

# --- Init ---
rm -rf "$DEMO_DIR" && mkdir -p "$DEMO_DIR" && cd "$DEMO_DIR"

header "Initialize workspace"
typeit "cn init"
pause

# --- Create tasks ---
header "Create tasks"
typeit 'cn task create "Design auth module" -p p0'
pause 0.4
typeit 'cn task create "Write integration tests" -p p1'
pause 0.4
typeit 'cn task create "Update API documentation" -p p2'
pause 0.4
typeit 'cn task create "Deploy to staging" -p p3'
pause

# --- List ---
header "List all tasks"
typeit "cn list"
pause 2

# --- Workflow ---
header "Claim and complete"
AUTH_ID=$(cn list 2>/dev/null | grep "Design auth" | head -1 | sed 's/[│ ]//g' | cut -c1-6)
typeit "cn claim $AUTH_ID"
pause 0.5
typeit "cn done $AUTH_ID --reason 'JWT + session design finalized'"
pause 0.5
typeit "cn list"
pause 2

# --- Detail ---
header "Task detail with event timeline"
typeit "cn show $AUTH_ID"
pause 3

# --- TUI ---
header "Interactive TUI dashboard"
echo -e "  \033[0;90mj/k navigate  Tab switch view  Enter detail  c claim  d done  q quit\033[0m"
echo ""
pause 1

# Drive TUI with expect
/usr/bin/expect <<'EOF'
  log_user 1
  set timeout 3
  spawn cn tui
  sleep 2.5
  send "j"
  sleep 0.6
  send "j"
  sleep 0.6
  send "\r"
  sleep 2
  send "\t"
  sleep 2.5
  send "q"
  sleep 0.5
  expect eof
EOF

pause 1

# --- Web ---
header "Web viewer"
echo -e "  \033[0;90mAxum + HTMX, auto-refresh, dark theme\033[0m"
echo ""
pause 0.8

cn serve --port 3905 &
SERVER_PID=$!
sleep 2

typeit "curl -s localhost:3905/api/tasks | python3 -m json.tool | head -20"
pause 2

kill $SERVER_PID 2>/dev/null
wait $SERVER_PID 2>/dev/null

echo ""
echo -e "  \033[1;36mchronis\033[0m  tasks as events, not rows."
echo -e "  \033[0;90mgithub.com/all-source-os/chronos-monorepo\033[0m"
echo ""
pause 3
