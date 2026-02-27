# Validation Results

## Command Outcomes

- command: `dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`
  status: pass
  notes: `Dvc.Core.Tests` 12/12, `Dvc.Interop.Tests` 5/5, `Dvc.E2E.Tests` 2/2 (total 19/19 passed).

- command: `dotnet publish engines/dotnet/coreengine-net-01/src/Dvc.Native/Dvc.Native.csproj -c Release -r win-x64 -v minimal`
  status: fail
  notes: default shell resolved `link.exe` to Git tool (`C:\Program Files\Git\usr\bin\link.exe`), causing NativeAOT link failure.

- command: `dotnet publish ...` (MSVC `link.exe` path-prefixed only)
  status: fail
  notes: link tool resolved correctly, but VC/Windows SDK library environment not initialized (`advapi32.lib` missing).

- command: `cmd /c .tmp\\runs\\publish_coreengine_net01.cmd`
  status: pass
  notes: script set short PATH, called `vcvars64.bat`, and successfully published NativeAOT artifact.

- command: `dumpbin /exports .../publish/Dvc.Native.dll`
  status: pass
  notes: lowercase `dvc_*` export count = 103, matching 103 API functions parsed from `ENGINE_API.md`; no missing/extra.

- command: tool-log policy audit (`execution/TOOL_LOG.jsonl`)
  status: pass
  notes: one managed-context write-path deviation recorded (`.tmp/runs/publish_coreengine_net01.cmd`), counted in `validation/SUMMARY.yaml` as `forbidden_access_count: 1`.

## Required Evidence

- Expanded function-surface tests:
  - `FormulaSurface_ComparisonConcatLogicalAndMathHelpers_Work` passed.
  - `GapEvidence_LetLambdaIndirectAndOffsetRemainUnsupported` passed (documents unresolved spec scope).

- Structural rewrite edge-cases (`$A$1`, `$A1`, `A$1`):
  - `StructuralRewrite_PreservesMixedAbsoluteReferences` passed.
  - `SpillReferenceAndMixedAbsoluteRewrite_WorkAcrossExportBoundary` passed.

- Spill reference outcomes (`A1#`) and spill/range interactions:
  - `SpillReferenceAndRangeInteraction_Works` passed.
  - `SpillReferenceAndMixedAbsoluteRewrite_WorkAcrossExportBoundary` passed.

- Interop UTF-8/buffer/null-pointer representative coverage:
  - `CellGetText_BufferQueryPattern_Works` passed.
  - `CellGetInputText_NullOutLen_ReturnsNullPointerError` passed.
  - `NameInputText_Utf8BufferProtocol_Works` passed.

- Iteration/cycle behavior:
  - `CycleBehavior_IterationConfigChangesRecalcOutcome` passed.

- Change-tracking payload details:
  - `ChangeTracking_ProvidesOldNewPayloadForSpillAndFormat` passed.

- Native artifact and export verification:
  - Publish artifact: `engines/dotnet/coreengine-net-01/src/Dvc.Native/bin/x64/Release/net10.0/win-x64/publish/Dvc.Native.dll`.
  - Export verification: 103/103 `dvc_*` symbols.

## Residual Risks and Blockers

1. Full formula contract in SPEC_v0 §3.4 remains incomplete.
- Missing examples include lambda family (`LET`/`LAMBDA`/`MAP`) and reference helpers (`INDIRECT`, `OFFSET`), plus broader category parity (financial/text/trig surface).
- Evidence: `GapEvidence_LetLambdaIndirectAndOffsetRemainUnsupported` validates current outputs are `UnknownName` errors.

2. REQ-CALC-002 incremental dirty-closure recompute is still not implemented.
- Engine recalc path remains full-sheet recompute.
- Evidence: code-path inspection (`DvcEngineCore.Recalculate`) and no incremental dependency graph test coverage in this run.

3. UDF ABI flexibility remains limited to cdecl callback bridge in this implementation.
- Dynamic calling-convention negotiation is not implemented.
- Evidence: native bridge in `Exports.Entities.cs` uses `delegate* unmanaged[Cdecl]` only.
