use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crossterm::event::KeyEvent;

use crate::{Action, AppMode};

#[derive(Debug)]
pub struct EventTrace {
    writer: BufWriter<File>,
    seq: u64,
}

impl EventTrace {
    pub fn open(path: &Path) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        Ok(Self {
            writer: BufWriter::new(file),
            seq: 0,
        })
    }

    pub fn log_key(
        &mut self,
        mode: AppMode,
        key: &KeyEvent,
        action: Option<&Action>,
    ) -> io::Result<()> {
        self.seq += 1;
        let ts_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let action = action
            .map(|value| format!("{value:?}"))
            .unwrap_or_else(|| "None".to_string());
        writeln!(
            self.writer,
            "seq={}\tts_ms={}\tmode={mode:?}\tkind={:?}\tcode={:?}\tmodifiers={:?}\tstate={:?}\taction={action}",
            self.seq, ts_ms, key.kind, key.code, key.modifiers, key.state
        )?;
        self.writer.flush()
    }
}

#[cfg(test)]
mod tests {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn writes_key_events_to_trace_log() {
        let temp = tempdir().expect("temp dir");
        let log_path = temp.path().join("events.log");
        let mut trace = EventTrace::open(&log_path).expect("open trace");
        let key = KeyEvent {
            code: KeyCode::Char('1'),
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        };

        trace
            .log_key(AppMode::Edit, &key, Some(&Action::InputChar('1')))
            .expect("write trace");

        let output = std::fs::read_to_string(log_path).expect("read trace");
        assert!(output.contains("mode=Edit"));
        assert!(output.contains("kind=Press"));
        assert!(output.contains("code=Char('1')"));
        assert!(output.contains("action=InputChar('1')"));
    }
}
