# Chronis TUI Improvements

Inspired by [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) — a rich TUI for browsing beads/issues with dependency graphs, fuzzy search, insights dashboards, and markdown rendering.

## Current State

- Dashboard (table + detail pane) and Kanban — 2 views
- j/k navigation, Enter for detail, c/d/a actions
- Detail pane shows: ID, title, priority, status, claimed_by, blocked_by, done_reason, description, timeline

## Improvements (priority order)

### 1. Fuzzy Search (`/`)

Add interactive fuzzy search over task ID, title, and description. Filter the task list in real-time as the user types. Esc to cancel, Enter to jump to match.

**Crate:** `nucleo` or `fuzzy-matcher`

### 2. Quick Status Filters (`o`/`p`/`d`)

One-key toggles to filter the dashboard table by status:
- `o` — open only
- `p` — in-progress only
- `d` — done only
- `x` — clear filter (show all)

Show active filter in the title bar.

### 3. Render Missing Metadata in Detail Pane

The `Task` struct has fields that aren't shown in the TUI detail panel:
- `task_type` (task/epic/bug/feature)
- `parent` (parent task ID)
- `created_at`
- `awaiting_approval` / `approved` / `approved_at`

Display all of these when present.

### 4. Dependency Graph View (`g`)

A third view showing the dependency DAG. Even a simple indented tree using `blocked_by` and `children_of` data. Highlight blocked vs. ready nodes with color.

### 5. Summary Stats in Title Bar

Replace the simple "N tasks" counter with a breakdown:
```
chronis  Dashboard  3 open | 2 in-progress | 5 done
```

### 6. Responsive Detail Pane

- Wrap long description text within the pane width
- On narrow terminals (< 100 cols), collapse to single-panel mode with Enter toggling a full-screen detail overlay

### 7. Live File Watcher

Use `notify` crate to watch `.chronis/` directory. Auto-refresh task list when events are appended by other agents or CLI commands. Remove the need for manual `r` refresh.

### 8. Clipboard Copy (`C`)

Copy the focused task as formatted markdown to the system clipboard.

**Crate:** `arboard`

### 9. Markdown Rendering in Descriptions

Render task descriptions with basic markdown formatting (headers, code blocks, lists, bold/italic).

**Crate:** `termimad`

### 10. Export (`E`)

Dump all tasks to a timestamped markdown file in the current directory. Include task metadata, descriptions, and dependency relationships.
