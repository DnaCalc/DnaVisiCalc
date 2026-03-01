# Run-Specific Additional Requirements

Run type:
- Export-interop and build-configuration improvement run.

Focus:
- Add a managed/JIT C ABI export path via DNNE-style native exports.
- Preserve NativeAOT path.

Must-have validation:
1. Build outputs:
   - managed/JIT export artifact path (DNNE-based),
   - NativeAOT export artifact path.
2. C API loadability:
   - `dnavisicalc-engine` must load each variant via explicit DLL path.
3. Conformance:
   - `cargo test -p dnavisicalc-engine --test conformance_smoke` passes for each variant when backend-pinned.

Non-goals:
- No broad formula-surface expansion unrelated to export/runtime mode.
- No repo-wide tooling rewrite.

Risk focus:
- export symbol parity drift between managed and AOT variants,
- runtime bootstrap/runtimeconfig requirements for managed variant,
- calling-convention mismatch or marshalling regressions.
