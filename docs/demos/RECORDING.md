# Recording the chronon Demo

## Prerequisites

```bash
# asciinema — terminal recorder
brew install asciinema

# agg — renders .cast → .gif (already installed via cargo)
cargo install --locked agg

# cn must be on PATH
cargo install --path apps/chronon
```

## Record

From the repo root:

```bash
asciinema rec --cols 110 --rows 32 docs/demos/chronon.cast -c "bash docs/demos/tui-demo.sh"
```

The script (`docs/demos/tui-demo.sh`) runs through:

1. `cn init` — initialize a workspace
2. `cn task create` — create 4 tasks at different priorities
3. `cn list` — show the task table
4. `cn claim` / `cn done` — full workflow with reason
5. `cn show` — task detail with event timeline
6. `cn tui` — interactive TUI (driven by expect: dashboard, navigate, detail, kanban, quit)
7. `cn serve` — web viewer with curl to JSON API

Total runtime: ~45 seconds.

## Render to GIF

```bash
agg docs/demos/chronon.cast docs/demos/chronon.gif --theme monokai --speed 1.5
```

Other theme options: `dracula`, `solarized-dark`, `nord`.

Adjust `--speed` to taste (1.0 = real-time, 2.0 = 2x).

## Upload to asciinema.org

```bash
asciinema upload docs/demos/chronon.cast
```

Returns a shareable URL like `https://asciinema.org/a/xxxxx`.

## Manual Recording (alternative)

If the automated script has issues with the TUI/expect portion, record manually:

```bash
asciinema rec --cols 110 --rows 32 docs/demos/chronon.cast
```

Then type the commands yourself:

```bash
cd /tmp && rm -rf cn-demo && mkdir cn-demo && cd cn-demo
cn init
cn task create "Design auth module" -p p0
cn task create "Write integration tests" -p p1
cn task create "Update API documentation" -p p2
cn task create "Deploy to staging" -p p3
cn list
cn claim <id>
cn done <id> --reason "JWT + session design finalized"
cn list
cn show <id>
cn tui          # navigate with j/k, Tab for kanban, Enter for detail, q to quit
cn serve        # Ctrl+C to stop
```

Press `Ctrl+D` to stop recording.

## Publishing to X

The thread copy is in `docs/demos/x-thread.md`. Attach the GIF to Tweet 1.

For best results on X:
- GIFs under 15MB get auto-played
- If the GIF is too large, upload the `.cast` to asciinema.org and link it instead
- Alternatively, use `agg --speed 2 --last-frame-duration 3` to trim the GIF size
