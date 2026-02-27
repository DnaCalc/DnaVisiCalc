# TUI Testability and Automation Contract

This document defines how the TUI is driven, observed, and replayed in deterministic tests and tooling.

## 1. Goals
- Drive the TUI end-to-end without manual terminal interaction.
- Capture full render state (content + style + cursor) at deterministic dimensions.
- Support scriptable key injection and replay traces.
- Produce durable artifacts (frame dumps, images, playback sessions) for debugging and regression review.

## 2. Implemented Seams

### Action reducer seam
- `App::apply(Action, &mut dyn WorkbookIo)` is the canonical behavior entrypoint.
- Tests can drive app behavior through explicit action streams.

### Key-input seam
- `action_from_key(AppMode, KeyEvent)` is the canonical key mapping path.
- `ScriptRunner::run_keys` drives the same key-to-action path as runtime input handling.

### Persistence seam
- `WorkbookIo` decouples save/open side effects from UI logic.
- `FsWorkbookIo` is runtime I/O.
- `MemoryWorkbookIo` is deterministic test I/O.

### Render/capture seam
- `capture::capture_app_frame(app, size)` captures deterministic full-color frame buffers.
- Capture includes per-row styled spans and optional cursor coordinates.
- Capture size is explicit (`CaptureSize { width, height }`).

### Artifact seam
- `capture_scenes` emits deterministic scene artifacts:
  - `.txt` frame text,
  - `.json` full styled frame payload with cursor,
  - `.svg` image export.
- `capture_script` executes keystroke scripts and emits timeline + per-frame artifacts.
- `capture_viewer` replays timeline artifacts in a CLI viewer.

## 3. Tool-Interaction Surface

### Fixed-size viewport runs
- Deterministic session size is caller-configurable.
- `capture_scenes` can use:
  - `DNAVISICALC_CAPTURE_WIDTH`
  - `DNAVISICALC_CAPTURE_HEIGHT`

### Keystroke scripting
- `capture_script <script-path> <output-dir> [width] [height]`.
- Script DSL:
  - `key <TOKEN>`
  - `text <STRING>`
  - `capture [LABEL]`

### Timeline playback
- `capture_viewer <timeline.json>` supports:
  - play/pause,
  - step +/-1,
  - jump +/-15,
  - speed control,
  - keystroke/action overlay.

## 4. Determinism Rules
- Deterministic mode must not depend on wall-clock timing for input ordering.
- Frame identity must be stable under identical key streams and viewport sizes.
- Input processing keeps key-kind filtering deterministic (`Press`/`Repeat` accepted, `Release` ignored).
- Volatile invalidation and stream ticks use separate runtime pathways.

## 5. Coverage Obligations
- Mode transitions and edit/command/paste flows.
- Command parsing including structural ops (`insrow`, `delrow`, `inscol`, `delcol` aliases).
- Global recalc trigger (`F9`) and command recalc parity (`:r` / `:recalc`) in auto/manual modes.
- Status behavior distinguishes valid-but-rejected commands from malformed command/usage errors.
- Render invariants at fixed viewport sizes.
- Capture/replay round-trip checks for scripted sessions.
- Artifact integrity checks for frame JSON and image exports.

## 6. Relationships
- Architecture alignment: [ARCHITECTURE.md](ARCHITECTURE.md)
- Test planning and rounds: [testing/TESTING_PLAN.md](testing/TESTING_PLAN.md)
