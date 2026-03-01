# Prompt Input

## Primary Instruction

You are executing a managed-context implementation run for:
- runtime: `dotnet`
- implementation id: `coreengine-net-01`
- target path: `engines/dotnet/coreengine-net-01`
- spec pack: `docs/full-engine-spec/2026-02-27`

Primary goal:
- Create a managed/JIT-export variant of the .NET core engine C ABI using DNNE (or equivalent DNNE export approach) so the C API can be loaded without NativeAOT publish-only artifacts.

Secondary goal:
- Keep NativeAOT export flow intact, and set up clear build configurations/targets for both variants.

Required outcomes:
1. Managed/JIT C ABI export artifact is produced and loadable by `dnavisicalc-engine` (`GetProcAddress` resolves required `dvc_*` exports).
2. NativeAOT export artifact remains buildable and loadable.
3. Build configuration surface is explicit and documented (for example: `managed-jit`, `native-aot`).
4. Backend-pinned conformance smoke passes against:
   - managed/JIT DNNE artifact,
   - NativeAOT artifact.
5. Existing behavior remains spec-compatible (no intentional semantic drift).

Scope constraints:
- Keep code edits inside:
  - `engines/dotnet/coreengine-net-01/**`
  - `runs/engine-impl/20260301-092202Z_dotnet_coreengine-net-01_2026-02-27/**`
- Do not edit Rust/OCaml engine implementations.

Validation evidence required:
- exact build commands for each variant,
- emitted artifact paths for each variant,
- conformance command/output for each variant,
- any interop caveats (loader deps, runtime prerequisites).

## Follow-up Instructions

1. If DNNE packaging details require explicit version pinning or bootstrap files, include that in project setup and documentation.
2. If a strict DNNE path is blocked by tooling/runtime constraints, document blocker concretely and provide the closest viable managed export approach with evidence.
