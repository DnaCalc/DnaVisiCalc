# Session Log

- [2026-03-01T07:54:00Z] Loaded run manifest, prompt input, additional requirements, context policy, issue artifacts, and 2026-02-27 engine spec/API docs.
- [2026-03-01T07:56:00Z] Inspected `DvcEngineCore` recalc/evaluator code and identified repeated parse + repeated recursive subgraph evaluation as primary recalc bottleneck.
- [2026-03-01T07:58:00Z] Captured baseline perf on current implementation with backend-pinned DLL (`before_tiny`, `before_moderate`).
- [2026-03-01T07:59:00Z] Recorded one managed-context read exception while checking prior run publish workaround (`.tmp/runs/publish_coreengine_net01.cmd`) for linker env setup.
- [2026-03-01T08:00:00Z] Implemented refactor: compiled-formula caching on inputs, parser decoupled from runtime resolvers, per-recalc memoization caches for cells/names, and allocation reduction in recalc traversal.
- [2026-03-01T08:01:00Z] Added core regression test to ensure shared dependency subgraphs are evaluated once per recalc pass.
- [2026-03-01T08:02:00Z] Ran .NET tests (`dotnet test engines/dotnet/coreengine-net-01/coreengine-net-01.slnx -v minimal`) and confirmed all tests passing.
- [2026-03-01T08:03:00Z] Published NativeAOT artifact with MSVC/SDK linker env to `src/Dvc.Native/bin/Release/net10.0/win-x64/publish/Dvc.Native.dll`.
- [2026-03-01T08:04:00Z] Captured post-refactor perf on tiny/moderate profiles (`after_tiny`, `after_moderate`).
- [2026-03-01T08:05:00Z] Ran backend-pinned conformance smoke (`cargo test -p dnavisicalc-engine --test conformance_smoke`) against published .NET DLL; pass.
- [2026-03-01T08:07:00Z] Updated validation, outputs, and handoff artifacts with exact commands, results, perf deltas, and residual risks.
