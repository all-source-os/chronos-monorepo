# ADR-010: Native ARM64 CI Runners

**Status:** Accepted
**Date:** 2026-02-03
**Release:** v0.10.0

## Context

Docker images were built using QEMU emulation for cross-compilation (x86_64 host → ARM64 target). This caused:
1. ~40 minute build times for the Erlang NIF compilation step
2. Intermittent QEMU crashes during heavy compilation
3. Apple Silicon developers couldn't build linux/amd64 images locally (QEMU fails on Erlang NIF)

## Decision

Replace QEMU-emulated multi-arch builds with native ARM64 GitHub Actions runners:

- Use `ubuntu-24.04-arm` runner label for ARM64 builds
- Build amd64 on standard `ubuntu-latest` runners
- Create multi-arch manifests from native builds
- Disable ARM64 builds temporarily when native runners are unavailable

## Consequences

### Positive
- Build time reduced from ~45 minutes to ~8 minutes
- No QEMU crashes — native compilation is deterministic
- Apple Silicon developers can build and test locally

### Negative
- Depends on GitHub ARM64 runner availability (not always available in all plans)
- Two separate build jobs instead of one multi-arch job
- Must maintain runner label compatibility across GitHub Actions updates
