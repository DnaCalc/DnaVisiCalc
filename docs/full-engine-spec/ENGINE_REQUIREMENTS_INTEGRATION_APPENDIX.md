# Engine Requirements Integration Appendix

This appendix keeps integration-facing requirements that were split out of `docs/ENGINE_REQUIREMENTS.md` to keep it strictly core-engine focused.

## 1. File Adapter Handoff (moved from former REQ-BULK-002)

Deterministic enumerators plus typed setters are sufficient for the current file adapter persistence scope:
- `MODE`,
- `ITER`,
- `DYNARR`,
- `CELL`,
- `NAME`,
- `CONTROL`,
- `CHART`,
- `FMT`.
