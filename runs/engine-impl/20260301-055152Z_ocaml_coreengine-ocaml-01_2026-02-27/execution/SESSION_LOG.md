# Session Log

Chronological human-readable log of key implementation steps and decisions.

- [2026-03-01T05:52:xxZ] Mapped issue artifact to normative requirements: `ENGINE_API.md` control/chart iterator contracts (`dvc_control_iterate`/`dvc_control_iterator_next`, `dvc_chart_iterate`/`dvc_chart_iterator_next`) and `ENGINE_REQUIREMENTS.md` `REQ-ENT-003` (create/remove/query/iterate surfaces).
- [2026-03-01T05:53:xxZ] Reproduced harness failure with pinned DLL (`ct_entities_001_control_roundtrip_holds`, `ct_entities_002_chart_roundtrip_holds`).
- [2026-03-01T05:56:xxZ] Added temporary trace instrumentation in `src/dvc_engine.c` to observe entity define/iterate behavior under failing tests.
- [2026-03-01T05:57:xxZ] Root cause confirmed: control/chart iterators advanced on length-probe calls (`name_buf == NULL`), which skips entries when caller uses probe-then-fetch.
- [2026-03-01T05:58:xxZ] Implemented behavior fix in `dvc_control_iterator_next` and `dvc_chart_iterator_next`: iterator index advances only after successful name copy; probe and buffer-too-small calls no longer consume entries.
- [2026-03-01T05:59:xxZ] Removed temporary tracing code.
- [2026-03-01T05:59:xxZ] Added local regression coverage in `tests/api_conformance_ct.c` (`CT-ENTITIES-001`, `CT-ENTITIES-002`) for two-step iterator usage.
- [2026-03-01T06:00:xxZ] Rebuilt OCaml DLL and reran pinned harness command: all conformance smoke tests passing (`13 passed, 0 failed`).
- [2026-03-01T06:01:xxZ] Rebuilt and ran local OCaml conformance binary (`dist/api_conformance_ct.exe`): new entity tests passed.
