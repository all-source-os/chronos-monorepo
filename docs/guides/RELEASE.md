# Release Guide

This guide covers the release workflow for the Chronos monorepo.

## Quick Reference

```bash
make release           # Full interactive release workflow
make release-quick     # Quick patch release (skip quality gates)
make release-preflight # Run pre-flight checks only
make version           # Show current version and recent tags
make images-check      # Check Docker image versions in GHCR
```

## Prerequisites

Before creating a release, ensure you have:

1. **Clean working tree**: No uncommitted changes
2. **GitHub CLI authenticated**: Run `gh auth login` if needed
3. **On main branch**: Releases should be created from main
4. **Remote accessible**: Can push to origin

The `make release-preflight` command checks all of these.

## Full Release Workflow

### 1. Start the Release

```bash
make release
```

### 2. Pre-flight Checks

The workflow automatically verifies:
- No uncommitted changes
- On main branch (or prompts for confirmation)
- Git remote is accessible
- GitHub CLI is authenticated

### 3. Version Selection

You'll be prompted to select a version:

```
Current version: v0.8.1

Version suggestions:
  1) v0.8.2 (patch - bug fixes)
  2) v0.9.0 (minor - new features)
  3) v1.0.0 (major - breaking changes)
  4) Custom version

Select version type (1-4) [1]:
```

**Version Guidelines:**
- **Patch (x.y.Z)**: Bug fixes, security patches, documentation updates
- **Minor (x.Y.0)**: New features, non-breaking changes
- **Major (X.0.0)**: Breaking changes, major rewrites

### 4. Release Title

Enter a descriptive title for the release:

```
Release title (e.g., 'Quality & Stability Release'):
```

### 5. Quality Gates (Optional)

You'll be asked whether to run quality gates:

```
Run quality gates before release? (Y/n):
```

This runs `make ci` which includes:
- Rust: formatting, Clippy, tests, docs
- Go: formatting, linting, tests
- Elixir: formatting, Credo, Dialyzer, tests

**Recommendation**: Always run quality gates for minor/major releases.

### 6. Documentation Updates

The workflow automatically updates:
- `README.md`: Version badges for Docker images
- `RELEASE.md`: Version number and release date

### 7. Git Tag Creation

An annotated tag is created with:
- Version number
- Release title
- Recent commit history

### 8. Push to Remote

The workflow pushes:
- Updated documentation to main branch
- New version tag (triggers docker-publish workflow)

### 9. GitHub Release

A GitHub release is created with:
- Release title
- Changelog (recent commits)
- Docker pull commands

### 10. Docker Image Publishing

The tag push triggers the `docker-publish` workflow which:
- Builds multi-arch images (amd64 + arm64)
- Pushes to GitHub Container Registry (ghcr.io)
- Tags images with version (e.g., `v0.8.2`, `0.8`, `latest`)

You can optionally wait for the workflow to complete.

## Quick Patch Release

For simple bug fixes, use the quick release:

```bash
make release-quick
```

This skips quality gates and uses defaults:
- Automatically increments patch version
- Uses "Patch Release" as default title
- Auto-generates release notes

## Post-Release Checklist

After a release:

1. **Verify Docker Images**
   ```bash
   make images-check
   ```
   Or visit: https://github.com/all-source-os/chronos-monorepo/pkgs

2. **Verify GitHub Release**
   Visit: https://github.com/all-source-os/chronos-monorepo/releases

3. **Update Release Branch** (optional)
   ```bash
   git checkout release/0.8
   git merge main
   git push origin release/0.8
   git checkout main
   ```

4. **Announce the Release**
   - Update project documentation
   - Notify stakeholders
   - Update changelog in external docs

## Troubleshooting

### Pre-flight Check Failures

**Uncommitted changes:**
```bash
git status
git stash  # or commit changes
make release
```

**Not on main branch:**
```bash
git checkout main
git pull origin main
make release
```

**GitHub CLI not authenticated:**
```bash
gh auth login
```

### Docker Publish Workflow Issues

If images aren't being tagged correctly:

1. Check workflow status:
   ```bash
   gh run list --workflow=docker-publish.yml --limit 5
   ```

2. View workflow logs:
   ```bash
   gh run view <run-id> --log
   ```

3. Manually trigger with force build:
   ```bash
   gh workflow run docker-publish.yml -f force_build=true
   ```

### Tag Already Exists

If you need to recreate a tag:

```bash
# Delete local tag
git tag -d v0.8.2

# Delete remote tag
git push origin :refs/tags/v0.8.2

# Recreate
make release
```

**Warning**: Only do this if the release hasn't been announced/used.

## Version History

| Version | Date | Highlights |
|---------|------|------------|
| v0.8.1 | 2026-02-08 | SIMD filter refactoring, Elixir test fixes |
| v0.8.0 | 2026-02-03 | Clean Architecture release |
| v0.7.3 | 2026-02-02 | Quality improvements |

## Related Documentation

- [Docker Images](../docker-images.md) - Container deployment guide
- [Quality Gates](./QUALITY_GATES_SETUP.md) - CI/CD quality checks
- [RELEASE.md](../../RELEASE.md) - Current release notes
