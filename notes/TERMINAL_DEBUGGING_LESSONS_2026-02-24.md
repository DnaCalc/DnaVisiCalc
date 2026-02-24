# TERMINAL_DEBUGGING_LESSONS_2026-02-24.md

## Context
- Issue observed: single keypress appears twice in the TUI (`1` becomes `11`).
- Goal: reproduce first, then fix.

## Lessons Learned

1. Repro-first discipline works.
- Adding trace scaffolding before changing behavior avoided guess-fixes.
- We confirmed duplication via captured key kinds (`Press` + `Release`) both mapping to input actions.

2. Terminal launch automation is fragile without strict quoting.
- `wt.exe` argument parsing can misroute title/command tokens and throw `0x80070002`.
- Safer approach:
  - explicit command string for `new-tab`,
  - explicit quoting for title and script path,
  - `--` separator before shell command.

3. Windows Terminal process behavior can invalidate naive waits.
- `wt.exe` can exit quickly after dispatching to an existing window.
- Waiting on the starter process (`HasExited`) is not a reliable readiness signal.
- Prefer activation/polling strategies tied to visible terminal windows.

4. Foreground activation needs fallback paths.
- `AppActivate` by custom title is not always reliable.
- A fallback activation target (`Windows Terminal`) improves robustness.

5. Event traces should be first-class artifacts.
- `DNAVISICALC_EVENT_TRACE` logs made the bug measurable and debuggable.
- Trace format should include mode, key kind, key code, modifiers, and mapped action.

6. UI-level automation and code-level repro should coexist.
- High-level harness (`wt + SendKeys`) validates real environment behavior.
- Code-level unit repro (mapping `Press` and `Release`) guarantees a deterministic local signal even when GUI automation is flaky.

7. Keep test harnesses opt-in when environment-dependent.
- Windows interactive tests should not run unconditionally in CI.
- Gate them with an env var and document prerequisites.

## Practical Guidance for Future Terminal Bugs

1. Enable event tracing first and capture a minimal failing sequence.
2. Add a deterministic unit/integration repro in code.
3. Add/extend an environment harness (terminal + key automation) only after step 2.
4. Do not apply behavioral fixes until both repro paths are in place.
5. After fix, rerun:
- unit repro,
- harness repro,
- full workspace tests.

## Open Follow-up
- Input policy likely needs key-kind filtering to prevent duplicate char insertion in terminals that emit both `Press` and `Release`.
- Keep this as a separate change after preserving repro evidence.
