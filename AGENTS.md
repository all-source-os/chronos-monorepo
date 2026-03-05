# Agent Instructions

This project supports **cn** (chronis) and **bd** (beads) for issue tracking. Chronis is the primary tracker. Run `cn init` to get started, or use `bd onboard` for legacy beads.

## Quick Reference (Chronis — preferred)

```bash
cn ready              # Find available work (open + unblocked)
cn show <id>          # View task details, children, timeline
cn claim <id>         # Claim a task (uses CN_AGENT_ID env var)
cn done <id>          # Complete a task
cn sync --git         # Sync with git (add .chronis/, commit, push)
```

## Quick Reference (Beads — legacy)

```bash
bd ready              # Find available work
bd show <id>          # View issue details
bd update <id> --status in_progress  # Claim work
bd close <id>         # Complete work
bd sync               # Sync with git
```

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   cn sync --git       # or: bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds


<!-- bv-agent-instructions-v1 -->

---

## Chronis Workflow Integration

This project uses [chronis](apps/chronis/) as the primary event-sourced task tracker. Tasks are stored in `.chronis/` and tracked in git.

### Essential Commands

```bash
# Task management
cn task create "Title" -p p0 --type=epic         # Create epic
cn task create "Subtask" --parent=<id> -p p1      # Create child task
cn task create "Bug fix" --type=bug               # Create bug
cn list                                           # All tasks
cn list --status=open                             # Filter by status
cn ready                                          # Unblocked open tasks
cn show <id>                                      # Details + children + timeline
cn claim <id>                                     # Claim (reads CN_AGENT_ID)
cn done <id> --reason="Completed"                 # Complete
cn approve <id>                                   # Approve

# Dependencies
cn dep add <task-id> <blocker-id>                 # Add blocker
cn dep remove <task-id> <blocker-id>              # Remove blocker

# Migration & sync
cn migrate-beads                                  # Import from .beads/
cn sync --git                                     # Git add/commit/push .chronis/

# Visualization
cn tui                                            # Interactive TUI
cn serve                                          # Web viewer (port 3905)
```

### Workflow Pattern

1. **Start**: Run `cn ready` to find actionable work
2. **Claim**: Use `cn claim <id>`
3. **Work**: Implement the task
4. **Complete**: Use `cn done <id>`
5. **Sync**: Always run `cn sync --git` at session end

### Key Concepts

- **Dependencies**: Tasks can block other tasks. `cn ready` shows only unblocked work.
- **Priority**: p0=critical, p1=high, p2=medium, p3=low
- **Types**: task, epic, bug, feature
- **Parent-child**: Use `--parent=<id>` to create hierarchy under epics
- **Blocking**: `cn dep add <task> <blocker>` to add post-creation dependencies

### Session Protocol

**Before ending any session, run this checklist:**

```bash
git status              # Check what changed
git add <files>         # Stage code changes
cn sync --git           # Commit chronis changes
git commit -m "..."     # Commit code
git push                # Push to remote
```

### Best Practices

- Check `cn ready` at session start to find available work
- Update status as you work (claim → done)
- Create new tasks with `cn task create` when you discover work
- Use descriptive titles and set appropriate priority/type
- Always `cn sync --git` before ending session

---

## Beads Workflow Integration (Legacy)

This project also has [beads_viewer](https://github.com/Dicklesworthstone/beads_viewer) for issue tracking. Issues are stored in `.beads/` and tracked in git.

### Essential Commands

```bash
# View issues (launches TUI - avoid in automated sessions)
bv

# CLI commands for agents (use these instead)
bd ready              # Show issues ready to work (no blockers)
bd list --status=open # All open issues
bd show <id>          # Full issue details with dependencies
bd create --title="..." --type=task --priority=2
bd update <id> --status=in_progress
bd close <id> --reason="Completed"
bd close <id1> <id2>  # Close multiple issues at once
bd sync               # Commit and push changes
```

### Key Concepts

- **Dependencies**: Issues can block other issues. `bd ready` shows only unblocked work.
- **Priority**: P0=critical, P1=high, P2=medium, P3=low, P4=backlog (use numbers, not words)
- **Types**: task, bug, feature, epic, question, docs
- **Blocking**: `bd dep add <issue> <depends-on>` to add dependencies

<!-- end-bv-agent-instructions -->
