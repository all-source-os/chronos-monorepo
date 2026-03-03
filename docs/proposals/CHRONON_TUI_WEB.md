# Plan: `cn tui` and `cn serve` — TUI Dashboard + Web Viewer

## Context

The chronon CLI was just refactored into clean architecture with a `TaskRepository` trait, typed domain models, and integration tests. Users migrating from beads lose the `beads_viewer` (`bv`) TUI experience. This adds two new commands: a ratatui TUI dashboard and an embedded web viewer, both consuming the existing `TaskRepository` API.

## New File Layout

```
apps/chronon/src/presentation/
  tui/
    mod.rs              # pub async fn run(), terminal setup/restore, panic hook
    app.rs              # App state: tasks, selected_index, view mode, status msg
    event.rs            # Event loop: crossterm poll + 1s tick auto-refresh
    ui.rs               # Top-level render: title bar, view dispatch, status bar
    views/
      mod.rs
      dashboard.rs      # Split-pane: task table (left) + detail+timeline (right)
      kanban.rs         # Three columns: Open | In-Progress | Done
  web/
    mod.rs              # pub async fn run(), Axum router, graceful shutdown
    handlers.rs         # Static assets, JSON API, HTMX partials, error type
    state.rs            # AppState wrapping Arc<CoreTaskRepository>
    assets/
      index.html        # Dashboard page with HTMX auto-refresh
      kanban.html       # Kanban board page
      style.css         # Dark theme, CSS grid layout
      htmx.min.js       # Embedded HTMX 2.0 (~14KB, offline-capable)
```

## Dependency Changes

### Workspace root `Cargo.toml`
- Add `"apps/chronon"` to `members`
- Add to `[workspace.dependencies]`: `clap = { version = "4.5", features = ["derive"] }`, `tabled = "0.17"`, `ratatui = "0.29"`, `crossterm = "0.28"`

### `apps/chronon/Cargo.toml`
- Add: `ratatui = { workspace = true }`, `crossterm = { workspace = true }`, `axum = { workspace = true }`, `tower-http = { workspace = true }`

## CLI Changes

Add to `Command` enum in `presentation/cli.rs`:
```rust
/// Launch interactive TUI dashboard
Tui,
/// Start embedded web viewer
Serve(ServeArgs),
```
`ServeArgs`: `--port` (default 3905), `--open` (auto-open browser)

## TUI Design (`cn tui`)

### State (`app.rs`)
- `App` holds: `Arc<CoreTaskRepository>`, `Vec<Task>`, `selected_index`, `View` (Dashboard|Kanban), `Focus`, `selected_detail: Option<TaskDetail>`, `should_quit`, `status_message`
- `refresh()` — sync call to `repo.list_tasks(None)` (DashMap, ~12μs)
- `tasks_by_status()` — partitions tasks for Kanban columns

### Event Loop (`event.rs`)
- `crossterm::event::poll(timeout)` with 1s tick for auto-refresh
- Async mutations (claim/done/approve) called with `.await` directly (we're in a tokio runtime)
- Keybindings: `j/k` navigate, `Enter` select, `Tab` switch view, `c` claim, `d` done, `a` approve, `q` quit

### Dashboard View (`views/dashboard.rs`)
- Horizontal split: 50/50
- Left: `ratatui::widgets::Table` with ID, Title, Pri, Status, Claimed columns, `TableState` for scroll
- Right: `Paragraph` block with task fields + timeline entries

### Kanban View (`views/kanban.rs`)
- Three equal columns via `Layout::horizontal`
- Each column: `List` widget with task cards (ID, title, priority, claimed_by)
- Active column highlighted, arrow keys switch columns

### Terminal Lifecycle (`mod.rs`)
```
ratatui::init() → panic hook → App::new() → refresh() → run_loop() → ratatui::restore()
```
Panic hook ensures terminal restoration if anything crashes.

## Web Viewer Design (`cn serve`)

### Router (`mod.rs`)
```
GET  /                      → index.html (dashboard)
GET  /kanban                → kanban.html
GET  /style.css             → embedded CSS
GET  /htmx.min.js           → embedded HTMX
GET  /api/tasks             → JSON list (optional ?status= filter)
GET  /api/tasks/:id         → JSON task detail + timeline
POST /api/tasks/:id/claim   → claim task
POST /api/tasks/:id/done    → complete task
POST /api/tasks/:id/approve → approve task
GET  /partials/task-list    → HTMX partial: task table rows
GET  /partials/task-detail/:id → HTMX partial: detail pane
GET  /partials/kanban       → HTMX partial: kanban columns
```

### Handlers (`handlers.rs`)
- Static assets served via `include_str!` with correct Content-Type
- JSON API returns `Json<Vec<Task>>` / `Json<TaskDetail>` (Task already derives Serialize)
- HTMX partials render HTML strings via `format!` (no template engine)
- `AppError` wraps `ChronError` → 404 for TaskNotFound, 409 for InvalidTransition, 500 for Core/Io

### Frontend (`assets/`)
- HTMX `hx-trigger="every 2s"` for auto-refresh task list
- Task row click → `hx-get="/partials/task-detail/{id}"` → swaps detail pane
- Action buttons POST → API → HX-Trigger header refreshes list
- Dark theme CSS (~150 lines), CSS Grid for layout
- HTMX embedded as static file (no CDN dependency, works offline)

## Data Flow

Both TUI and web use identical patterns:
```
Workspace::open().await → ws.repo() → Arc<CoreTaskRepository>
  ├── list_tasks(status)      [sync, DashMap projection, ~12μs]
  ├── ready_tasks()            [sync]
  ├── get_task(id)             [sync]
  ├── get_task_detail(id)      [async, queries WAL for timeline]
  ├── claim_task(id, agent)    [async, ingests event]
  ├── complete_task(id, reason)[async, ingests event]
  └── approve_task(id)         [async, ingests event]
```

## main.rs Wiring

```rust
Command::Tui => {
    let ws = Workspace::open().await?;
    let repo = ws.repo();
    tui::run(repo).await?;    // blocks until quit
    ws.shutdown().await?;
}
Command::Serve(args) => {
    let ws = Workspace::open().await?;
    let repo = ws.repo();
    web::run(repo, args.port).await?;  // blocks until ctrl-c
    ws.shutdown().await?;
}
```

## Implementation Steps

1. Workspace Cargo.toml — add chronon to members, add clap/tabled/ratatui/crossterm
2. Chronon Cargo.toml — add ratatui, crossterm, axum, tower-http
3. `presentation/cli.rs` — add Tui and Serve variants
4. `presentation/tui/` — app.rs → event.rs → views/ → ui.rs → mod.rs
5. `presentation/web/` — state.rs → assets/ → handlers.rs → mod.rs
6. `main.rs` — add match arms
7. Build, test, fix

## Verification

```bash
cargo build -p chronon
cargo test -p chronon        # existing 8 tests still pass

# TUI smoke test
cd /tmp && mkdir test-tui && cd test-tui
cn init && cn task create "Alpha" -p p0 && cn task create "Beta" -p p1
cn tui                       # verify dashboard + kanban views, claim/done from TUI

# Web smoke test
cn serve --port 3905         # open http://localhost:3905
# verify: task list renders, kanban view works, claim/done via buttons, auto-refresh
```
