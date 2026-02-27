# Full Engine Spec Packs

This directory stores immutable, dated core-engine spec packs.

## Pack Index

- `2026-02-27` (current baseline)

## Versioning Rule

- Each spec pack is frozen under a date directory (`YYYY-MM-DD`).
- Future changes create a new dated directory; existing dated packs are not edited.
- Consumers should pin to a specific dated pack path.

## Current Baseline Pack

- Path: `docs/full-engine-spec/2026-02-27`
- Scope: core engine contract (`SPEC_v0`, `ENGINE_REQUIREMENTS`, `ENGINE_API`) plus informative appendices.

## Note on Top-Level Mirror Files

Top-level files in `docs/full-engine-spec/` currently mirror the latest baseline for convenience.
For immutable consumption, use the dated directory path only.
