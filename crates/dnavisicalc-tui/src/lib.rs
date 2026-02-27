pub mod app;
pub mod capture;
mod event_trace;
pub mod io;
pub mod keymap;
pub mod render;
pub mod runtime;

pub use app::{Action, App, AppMode, CommandOutcome, ControlKind, PanelControl, SpillRole};
pub use capture::{
    CaptureCursor, CaptureFrame, CaptureRow, CaptureSize, CaptureSpan, CaptureTimeline,
    TimelineFrame, capture_app_frame, frame_to_text, write_frame_json, write_frame_svg,
    write_frame_text,
};
pub use io::{FsWorkbookIo, MemoryWorkbookIo, WorkbookIo};
pub use keymap::action_from_key;
pub use render::render_app;
pub use runtime::{RuntimeOptions, run_from_env, run_with_options};
