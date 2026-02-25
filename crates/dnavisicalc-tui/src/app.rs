use std::collections::{HashMap, HashSet};

use dnavisicalc_core::{
    CellFormat, CellInput, CellRef, Engine, PaletteColor, RecalcMode, Value, col_index_to_label,
};

use crate::io::WorkbookIo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Navigate,
    Edit,
    Command,
    PasteSpecial,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    ExtendLeft,
    ExtendRight,
    ExtendUp,
    ExtendDown,
    ClearSelection,
    ToggleHelp,
    CopySelection,
    PasteFromClipboard,
    BeginPasteFromClipboard(String),
    PasteModeNext,
    PasteModePrev,
    StartEdit,
    StartCommand,
    InputChar(char),
    Backspace,
    Submit,
    Cancel,
    Recalculate,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpillRole {
    None,
    Anchor,
    Member,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandOutcome {
    Continue,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMode {
    All,
    Formulas,
    Values,
    ValuesKeepDestinationFormatting,
    Formatting,
}

impl PasteMode {
    pub const ALL: [PasteMode; 5] = [
        PasteMode::All,
        PasteMode::Formulas,
        PasteMode::Values,
        PasteMode::ValuesKeepDestinationFormatting,
        PasteMode::Formatting,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Formulas => "Formulas",
            Self::Values => "Values",
            Self::ValuesKeepDestinationFormatting => "Values+KeepDestFmt",
            Self::Formatting => "Formatting",
        }
    }
}

#[derive(Debug, Clone)]
struct CopyCell {
    input: Option<CellInput>,
    value: Value,
    format: CellFormat,
}

#[derive(Debug, Clone)]
struct CopyBuffer {
    width: u16,
    height: u16,
    cells: Vec<CopyCell>,
    text: String,
}

#[derive(Debug, Clone)]
pub struct App {
    engine: Engine,
    mode: AppMode,
    selected: CellRef,
    selection_anchor: Option<CellRef>,
    viewport_col: u16,
    viewport_row: u16,
    viewport_width: u16,
    viewport_height: u16,
    edit_buffer: String,
    command_buffer: String,
    pending_paste_text: Option<String>,
    paste_mode_index: usize,
    copy_buffer: Option<CopyBuffer>,
    last_copy_text: Option<String>,
    status: String,
    last_path: Option<String>,
    last_saved_epoch: Option<u64>,
    help_visible: bool,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        let engine = Engine::new();
        let selected = CellRef::from_a1("A1").expect("A1 must be valid");
        Self {
            engine,
            mode: AppMode::Navigate,
            selected,
            selection_anchor: None,
            viewport_col: 1,
            viewport_row: 1,
            viewport_width: 8,
            viewport_height: 12,
            edit_buffer: String::new(),
            command_buffer: String::new(),
            pending_paste_text: None,
            paste_mode_index: 0,
            copy_buffer: None,
            last_copy_text: None,
            status: "Ready".to_string(),
            last_path: None,
            last_saved_epoch: None,
            help_visible: false,
        }
    }

    pub fn from_engine(engine: Engine) -> Self {
        let mut app = Self::new();
        app.engine = engine;
        app.last_saved_epoch = Some(app.engine.committed_epoch());
        app
    }

    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    pub fn mode(&self) -> AppMode {
        self.mode
    }

    pub fn selected_cell(&self) -> CellRef {
        self.selected
    }

    pub fn selected_range(&self) -> dnavisicalc_core::CellRange {
        let anchor = self.selection_anchor.unwrap_or(self.selected);
        dnavisicalc_core::CellRange::new(anchor, self.selected)
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn current_path(&self) -> Option<&str> {
        self.last_path.as_deref()
    }

    pub fn help_visible(&self) -> bool {
        self.help_visible
    }

    pub fn save_state_label(&self) -> &'static str {
        match (self.last_saved_epoch, self.is_dirty()) {
            (None, false) => "never saved",
            (None, true) => "unsaved changes",
            (Some(_), false) => "saved",
            (Some(_), true) => "modified",
        }
    }

    pub fn viewport_dimensions(&self) -> (u16, u16) {
        (self.viewport_width, self.viewport_height)
    }

    pub fn set_viewport_dimensions(&mut self, width: u16, height: u16) {
        self.viewport_width = width.max(1);
        self.viewport_height = height.max(1);
        self.ensure_visible();
    }

    pub fn edit_buffer(&self) -> &str {
        &self.edit_buffer
    }

    pub fn command_buffer(&self) -> &str {
        &self.command_buffer
    }

    pub fn last_copy_text(&self) -> Option<&str> {
        self.last_copy_text.as_deref()
    }

    pub fn paste_mode(&self) -> Option<PasteMode> {
        if self.mode == AppMode::PasteSpecial {
            Some(PasteMode::ALL[self.paste_mode_index])
        } else {
            None
        }
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    fn is_dirty(&self) -> bool {
        match self.last_saved_epoch {
            Some(saved_epoch) => self.engine.committed_epoch() != saved_epoch,
            None => self.engine.committed_epoch() > 0,
        }
    }

    pub fn apply(&mut self, action: Action, io: &mut dyn WorkbookIo) -> CommandOutcome {
        match self.mode {
            AppMode::Navigate => self.apply_navigate(action, io),
            AppMode::Edit => self.apply_edit(action),
            AppMode::Command => self.apply_command(action, io),
            AppMode::PasteSpecial => self.apply_paste_special(action),
        }
    }

    pub fn visible_grid(&self, width: u16, height: u16) -> GridSnapshot {
        let mut headers = Vec::new();
        for offset in 0..width {
            headers.push(col_index_to_label(self.viewport_col + offset));
        }

        let mut rows = Vec::new();
        for row_offset in 0..height {
            let row_num = self.viewport_row + row_offset;
            let mut cells = Vec::new();
            for col_offset in 0..width {
                let col_num = self.viewport_col + col_offset;
                let cell = CellRef {
                    col: col_num,
                    row: row_num,
                };
                let active = cell == self.selected;
                let in_selection = self.selection_contains(cell);
                let (value, is_text) = self
                    .engine
                    .cell_state(cell)
                    .map(|state| {
                        let format = self.engine.cell_format(cell).unwrap_or_default();
                        (
                            format_value(&state.value, format.decimals),
                            matches!(state.value, Value::Text(_)),
                        )
                    })
                    .unwrap_or_else(|_| ("#ADDR".to_string(), false));
                let spill_role = match self.engine.spill_range_for_cell(cell).ok().flatten() {
                    Some(range) if range.start == cell => SpillRole::Anchor,
                    Some(_) => SpillRole::Member,
                    None => SpillRole::None,
                };
                let format = self.engine.cell_format(cell).unwrap_or_default();
                cells.push(GridCell {
                    active,
                    in_selection,
                    value,
                    is_text,
                    spill_role,
                    format,
                });
            }
            rows.push(GridRow {
                row_label: row_num,
                cells,
            });
        }

        GridSnapshot { headers, rows }
    }

    pub fn evaluate_display_for_selected(&self) -> String {
        let format = self.engine.cell_format(self.selected).unwrap_or_default();
        self.engine
            .cell_state(self.selected)
            .map(|state| format_value(&state.value, format.decimals))
            .unwrap_or_else(|err| format!("#ERR {err}"))
    }

    pub fn spill_info_for_selected(&self) -> Option<String> {
        let range = self
            .engine
            .spill_range_for_cell(self.selected)
            .ok()
            .flatten()?;
        Some(format!("Spill {}", format_range(range)))
    }

    pub fn formula_or_input_for_selected(&self) -> String {
        match self.engine.cell_input(self.selected) {
            Ok(Some(CellInput::Formula(formula))) => formula,
            Ok(Some(CellInput::Number(n))) => n.to_string(),
            Ok(Some(CellInput::Text(text))) => text,
            Ok(None) => String::new(),
            Err(_) => String::new(),
        }
    }

    fn apply_navigate(&mut self, action: Action, io: &mut dyn WorkbookIo) -> CommandOutcome {
        match action {
            Action::MoveLeft => self.move_selection(-1, 0, false),
            Action::MoveRight => self.move_selection(1, 0, false),
            Action::MoveUp => self.move_selection(0, -1, false),
            Action::MoveDown => self.move_selection(0, 1, false),
            Action::ExtendLeft => self.move_selection(-1, 0, true),
            Action::ExtendRight => self.move_selection(1, 0, true),
            Action::ExtendUp => self.move_selection(0, -1, true),
            Action::ExtendDown => self.move_selection(0, 1, true),
            Action::ClearSelection => {
                let cells = self.selection_cells();
                if let Err(err) = self.clear_cells(cells) {
                    self.status = err;
                } else {
                    self.status = format!("Cleared {}", self.selection_label());
                }
            }
            Action::ToggleHelp => {
                self.help_visible = !self.help_visible;
                self.status = if self.help_visible {
                    "Help shown (press ? to close)".to_string()
                } else {
                    "Help hidden".to_string()
                };
            }
            Action::CopySelection => match self.capture_copy_buffer() {
                Ok(()) => self.status = format!("Copied {}", self.selection_label()),
                Err(err) => self.status = format!("Copy error: {err}"),
            },
            Action::BeginPasteFromClipboard(text) => {
                if text.trim().is_empty() {
                    self.status = "Clipboard is empty".to_string();
                } else {
                    self.pending_paste_text = Some(text);
                    self.paste_mode_index = 0;
                    self.mode = AppMode::PasteSpecial;
                    self.status =
                        "Paste Special: choose mode (1-5/Tab, Enter apply, Esc cancel)".to_string();
                }
            }
            Action::PasteFromClipboard => {
                self.status = "Paste failed: clipboard payload unavailable".to_string();
            }
            Action::StartEdit => {
                if let Some(reason) = self.editing_block_reason(self.selected) {
                    self.status = reason;
                    return CommandOutcome::Continue;
                }
                self.mode = AppMode::Edit;
                self.edit_buffer = self.formula_or_input_for_selected();
                self.status = format!("Edit {}", self.selected);
            }
            Action::StartCommand => {
                self.mode = AppMode::Command;
                self.command_buffer.clear();
                self.status = "Command mode".to_string();
            }
            Action::Recalculate => match self.engine.recalculate() {
                Ok(()) => self.status = "Recalculated".to_string(),
                Err(err) => self.status = format!("Recalc error: {err}"),
            },
            Action::Quit => return CommandOutcome::Quit,
            Action::PasteModeNext
            | Action::PasteModePrev
            | Action::InputChar(_)
            | Action::Backspace
            | Action::Submit
            | Action::Cancel => {}
        }

        self.ensure_visible();
        let _ = io;
        CommandOutcome::Continue
    }

    fn apply_edit(&mut self, action: Action) -> CommandOutcome {
        match action {
            Action::InputChar(ch) => {
                self.edit_buffer.push(ch);
            }
            Action::Backspace => {
                self.edit_buffer.pop();
            }
            Action::Cancel => {
                self.mode = AppMode::Navigate;
                self.status = "Edit cancelled".to_string();
            }
            Action::Submit => {
                let result = self.apply_edit_buffer();
                self.mode = AppMode::Navigate;
                self.status = result;
            }
            Action::MoveLeft
            | Action::MoveRight
            | Action::MoveUp
            | Action::MoveDown
            | Action::ExtendLeft
            | Action::ExtendRight
            | Action::ExtendUp
            | Action::ExtendDown
            | Action::ClearSelection
            | Action::CopySelection
            | Action::PasteFromClipboard
            | Action::BeginPasteFromClipboard(_)
            | Action::PasteModeNext
            | Action::PasteModePrev
            | Action::ToggleHelp
            | Action::StartEdit
            | Action::StartCommand
            | Action::Recalculate
            | Action::Quit => {}
        }
        CommandOutcome::Continue
    }

    fn apply_command(&mut self, action: Action, io: &mut dyn WorkbookIo) -> CommandOutcome {
        match action {
            Action::InputChar(ch) => self.command_buffer.push(ch),
            Action::Backspace => {
                self.command_buffer.pop();
            }
            Action::Cancel => {
                self.mode = AppMode::Navigate;
                self.status = "Command cancelled".to_string();
            }
            Action::Submit => {
                let cmd = self.command_buffer.trim().to_string();
                self.command_buffer.clear();
                self.mode = AppMode::Navigate;
                return self.execute_command(&cmd, io);
            }
            Action::MoveLeft
            | Action::MoveRight
            | Action::MoveUp
            | Action::MoveDown
            | Action::ExtendLeft
            | Action::ExtendRight
            | Action::ExtendUp
            | Action::ExtendDown
            | Action::ClearSelection
            | Action::CopySelection
            | Action::PasteFromClipboard
            | Action::BeginPasteFromClipboard(_)
            | Action::PasteModeNext
            | Action::PasteModePrev
            | Action::ToggleHelp
            | Action::StartEdit
            | Action::StartCommand
            | Action::Recalculate
            | Action::Quit => {}
        }
        CommandOutcome::Continue
    }

    fn apply_paste_special(&mut self, action: Action) -> CommandOutcome {
        match action {
            Action::Cancel => {
                self.mode = AppMode::Navigate;
                self.pending_paste_text = None;
                self.status = "Paste cancelled".to_string();
            }
            Action::Submit => {
                let mode = PasteMode::ALL[self.paste_mode_index];
                match self.apply_paste(mode) {
                    Ok(()) => {
                        self.mode = AppMode::Navigate;
                        self.pending_paste_text = None;
                        self.status = format!("Pasted {} to {}", mode.label(), self.selected);
                    }
                    Err(err) => {
                        self.mode = AppMode::Navigate;
                        self.pending_paste_text = None;
                        self.status = format!("Paste error: {err}");
                    }
                }
            }
            Action::PasteModeNext | Action::MoveDown | Action::MoveRight | Action::ExtendDown => {
                self.paste_mode_index = (self.paste_mode_index + 1) % PasteMode::ALL.len();
            }
            Action::PasteModePrev | Action::MoveUp | Action::MoveLeft | Action::ExtendUp => {
                if self.paste_mode_index == 0 {
                    self.paste_mode_index = PasteMode::ALL.len() - 1;
                } else {
                    self.paste_mode_index -= 1;
                }
            }
            Action::InputChar(ch) => {
                if ('1'..='5').contains(&ch) {
                    self.paste_mode_index = (ch as usize) - ('1' as usize);
                } else if ch == '\t' {
                    self.paste_mode_index = (self.paste_mode_index + 1) % PasteMode::ALL.len();
                }
            }
            Action::CopySelection
            | Action::PasteFromClipboard
            | Action::BeginPasteFromClipboard(_)
            | Action::ToggleHelp
            | Action::StartEdit
            | Action::StartCommand
            | Action::Backspace
            | Action::Recalculate
            | Action::Quit
            | Action::ClearSelection
            | Action::ExtendLeft
            | Action::ExtendRight => {}
        }
        CommandOutcome::Continue
    }

    fn execute_command(&mut self, cmd: &str, io: &mut dyn WorkbookIo) -> CommandOutcome {
        if cmd.is_empty() {
            self.status = "No command".to_string();
            return CommandOutcome::Continue;
        }

        let mut parts = cmd.split_whitespace();
        let Some(name) = parts.next() else {
            self.status = "No command".to_string();
            return CommandOutcome::Continue;
        };

        match name {
            "q" | "quit" => return CommandOutcome::Quit,
            "r" | "recalc" => match self.engine.recalculate() {
                Ok(()) => self.status = "Recalculated".to_string(),
                Err(err) => self.status = format!("Recalc error: {err}"),
            },
            "mode" => {
                let Some(mode_arg) = parts.next() else {
                    self.status = "Usage: mode auto|manual".to_string();
                    return CommandOutcome::Continue;
                };
                match mode_arg {
                    "auto" => {
                        self.engine.set_recalc_mode(RecalcMode::Automatic);
                        if let Err(err) = self.engine.recalculate() {
                            self.status = format!("Mode switch recalc error: {err}");
                        } else {
                            self.status = "Mode: automatic".to_string();
                        }
                    }
                    "manual" => {
                        self.engine.set_recalc_mode(RecalcMode::Manual);
                        self.status = "Mode: manual".to_string();
                    }
                    _ => {
                        self.status = "Usage: mode auto|manual".to_string();
                    }
                }
            }
            "w" | "write" => {
                let path = parts
                    .next()
                    .map(ToString::to_string)
                    .or_else(|| self.last_path.clone());
                let Some(path) = path else {
                    self.status = "Usage: w <path>".to_string();
                    return CommandOutcome::Continue;
                };
                match io.save(&path, &self.engine) {
                    Ok(()) => {
                        self.last_path = Some(path.clone());
                        self.last_saved_epoch = Some(self.engine.committed_epoch());
                        self.status = format!("Saved {path}");
                    }
                    Err(err) => self.status = format!("Save error: {err}"),
                }
            }
            "o" | "open" => {
                let Some(path) = parts.next() else {
                    self.status = "Usage: o <path>".to_string();
                    return CommandOutcome::Continue;
                };
                match io.load(path) {
                    Ok(engine) => {
                        self.engine = engine;
                        self.last_path = Some(path.to_string());
                        self.last_saved_epoch = Some(self.engine.committed_epoch());
                        self.status = format!("Loaded {path}");
                    }
                    Err(err) => self.status = format!("Open error: {err}"),
                }
            }
            "set" => {
                let Some(cell) = parts.next() else {
                    self.status = "Usage: set <A1> <value|formula>".to_string();
                    return CommandOutcome::Continue;
                };
                let parsed_cell = match CellRef::from_a1_with_bounds(cell, self.engine.bounds()) {
                    Ok(value) => value,
                    Err(err) => {
                        self.status = format!("Set error: {err}");
                        return CommandOutcome::Continue;
                    }
                };
                if let Some(reason) = self.editing_block_reason(parsed_cell) {
                    self.status = reason;
                    return CommandOutcome::Continue;
                }
                let value = parts.collect::<Vec<_>>().join(" ");
                if value.trim().is_empty() {
                    self.status = "Usage: set <A1> <value|formula>".to_string();
                    return CommandOutcome::Continue;
                }
                if let Err(err) = apply_input_to_cell(&mut self.engine, cell, value.trim()) {
                    self.status = format!("Set error: {err}");
                } else {
                    self.status = format!("Set {cell}");
                }
            }
            "name" => {
                let Some(arg1) = parts.next() else {
                    self.status =
                        "Usage: name <NAME> <value|formula> | name clear <NAME>".to_string();
                    return CommandOutcome::Continue;
                };
                if arg1.eq_ignore_ascii_case("clear") {
                    let Some(name) = parts.next() else {
                        self.status =
                            "Usage: name <NAME> <value|formula> | name clear <NAME>".to_string();
                        return CommandOutcome::Continue;
                    };
                    match self.engine.clear_name(name) {
                        Ok(()) => {
                            self.status = format!("Cleared name {}", name.to_ascii_uppercase())
                        }
                        Err(err) => self.status = format!("Name error: {err}"),
                    }
                    return CommandOutcome::Continue;
                }

                let name = arg1;
                let value = parts.collect::<Vec<_>>().join(" ");
                if value.trim().is_empty() {
                    self.status =
                        "Usage: name <NAME> <value|formula> | name clear <NAME>".to_string();
                    return CommandOutcome::Continue;
                }
                match apply_input_to_name(&mut self.engine, name, value.trim()) {
                    Ok(()) => self.status = format!("Set name {}", name.to_ascii_uppercase()),
                    Err(err) => self.status = format!("Name error: {err}"),
                }
            }
            "fmt" => {
                let Some(kind) = parts.next() else {
                    self.status = "Usage: fmt decimals <0..9|none> | fmt bold on|off | fmt italic on|off | fmt fg <color|none> | fmt bg <color|none> | fmt clear".to_string();
                    return CommandOutcome::Continue;
                };
                let cells = self.selection_cells();
                let result = match kind {
                    "decimals" => match parts.next() {
                        Some(value) if value.eq_ignore_ascii_case("none") => {
                            self.apply_format(cells, |format| {
                                format.decimals = None;
                            })
                        }
                        Some(value) => match value.parse::<u8>() {
                            Ok(v) if v <= 9 => self.apply_format(cells, |format| {
                                format.decimals = Some(v);
                            }),
                            _ => Err("Usage: fmt decimals <0..9|none>".to_string()),
                        },
                        None => Err("Usage: fmt decimals <0..9|none>".to_string()),
                    },
                    "bold" => match parts.next().and_then(parse_on_off) {
                        Some(flag) => self.apply_format(cells, |format| {
                            format.bold = flag;
                        }),
                        None => Err("Usage: fmt bold on|off".to_string()),
                    },
                    "italic" => match parts.next().and_then(parse_on_off) {
                        Some(flag) => self.apply_format(cells, |format| {
                            format.italic = flag;
                        }),
                        None => Err("Usage: fmt italic on|off".to_string()),
                    },
                    "fg" => match parts.next() {
                        Some(value) if value.eq_ignore_ascii_case("none") => {
                            self.apply_format(cells, |format| {
                                format.fg = None;
                            })
                        }
                        Some(value) => match PaletteColor::from_name(value) {
                            Some(color) => self.apply_format(cells, |format| {
                                format.fg = Some(color);
                            }),
                            None => Err("Usage: fmt fg <color|none>".to_string()),
                        },
                        None => Err("Usage: fmt fg <color|none>".to_string()),
                    },
                    "bg" => match parts.next() {
                        Some(value) if value.eq_ignore_ascii_case("none") => {
                            self.apply_format(cells, |format| {
                                format.bg = None;
                            })
                        }
                        Some(value) => match PaletteColor::from_name(value) {
                            Some(color) => self.apply_format(cells, |format| {
                                format.bg = Some(color);
                            }),
                            None => Err("Usage: fmt bg <color|none>".to_string()),
                        },
                        None => Err("Usage: fmt bg <color|none>".to_string()),
                    },
                    "clear" => self.apply_format(cells, |format| {
                        *format = CellFormat::default();
                    }),
                    _ => Err("Usage: fmt decimals|bold|italic|fg|bg|clear ...".to_string()),
                };
                match result {
                    Ok(()) => self.status = format!("Formatted {}", self.selection_label()),
                    Err(err) => self.status = format!("Format error: {err}"),
                }
            }
            "help" | "?" => {
                self.help_visible = true;
                self.status = "Help shown (press ? to close)".to_string();
            }
            _ => {
                self.status = format!("Unknown command: {name}");
            }
        }

        CommandOutcome::Continue
    }

    fn apply_edit_buffer(&mut self) -> String {
        if let Some(reason) = self.editing_block_reason(self.selected) {
            return reason;
        }
        let input = self.edit_buffer.trim();
        if input.is_empty() {
            return match self.engine.clear_cell(self.selected) {
                Ok(()) => format!("Cleared {}", self.selected),
                Err(err) => format!("Clear error: {err}"),
            };
        }

        match apply_input_to_cell(&mut self.engine, &self.selected.to_string(), input) {
            Ok(()) => format!("Set {}", self.selected),
            Err(err) => format!("Set error: {err}"),
        }
    }

    fn move_selection(&mut self, dx: i16, dy: i16, extend: bool) {
        if extend {
            if self.selection_anchor.is_none() {
                self.selection_anchor = Some(self.selected);
            }
        } else {
            self.selection_anchor = None;
        }
        let bounds = self.engine.bounds();
        let next_col = (self.selected.col as i16 + dx).clamp(1, bounds.max_columns as i16) as u16;
        let next_row = (self.selected.row as i16 + dy).clamp(1, bounds.max_rows as i16) as u16;
        self.selected.col = next_col;
        self.selected.row = next_row;
    }

    fn ensure_visible(&mut self) {
        let width = self.viewport_width.max(1);
        let height = self.viewport_height.max(1);
        if self.selected.col < self.viewport_col {
            self.viewport_col = self.selected.col;
        }
        if self.selected.row < self.viewport_row {
            self.viewport_row = self.selected.row;
        }
        if self.selected.col >= self.viewport_col + width {
            self.viewport_col = self.selected.col - width + 1;
        }
        if self.selected.row >= self.viewport_row + height {
            self.viewport_row = self.selected.row - height + 1;
        }
    }

    fn selection_contains(&self, cell: CellRef) -> bool {
        let range = self.selected_range();
        cell.col >= range.start.col
            && cell.col <= range.end.col
            && cell.row >= range.start.row
            && cell.row <= range.end.row
    }

    fn selection_cells(&self) -> Vec<CellRef> {
        self.selected_range().iter().collect()
    }

    fn selection_label(&self) -> String {
        let range = self.selected_range();
        if range.start == range.end {
            range.start.to_string()
        } else {
            format!("{}:{}", range.start, range.end)
        }
    }

    fn clear_cells(&mut self, cells: Vec<CellRef>) -> Result<(), String> {
        let selected_set: HashSet<CellRef> = cells.iter().copied().collect();
        for cell in &cells {
            if let Some(anchor) = self
                .engine
                .spill_anchor_for_cell(*cell)
                .map_err(|err| err.to_string())?
            {
                if !selected_set.contains(&anchor) {
                    return Err(format!(
                        "Cannot clear spilled cell {cell}; include anchor {anchor}"
                    ));
                }
            }
        }

        for cell in cells {
            if self
                .engine
                .spill_anchor_for_cell(cell)
                .map_err(|err| err.to_string())?
                .is_some()
            {
                continue;
            }
            self.engine
                .clear_cell(cell)
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn apply_format<F>(&mut self, cells: Vec<CellRef>, mut updater: F) -> Result<(), String>
    where
        F: FnMut(&mut CellFormat),
    {
        for cell in cells {
            let mut format = self
                .engine
                .cell_format(cell)
                .map_err(|err| err.to_string())?;
            updater(&mut format);
            self.engine
                .set_cell_format(cell, format)
                .map_err(|err| err.to_string())?;
        }
        Ok(())
    }

    fn capture_copy_buffer(&mut self) -> Result<(), String> {
        let range = self.selected_range();
        let width = range.end.col - range.start.col + 1;
        let height = range.end.row - range.start.row + 1;

        let mut cells = Vec::with_capacity((width as usize) * (height as usize));
        let mut lines: Vec<String> = Vec::with_capacity(height as usize);
        for row in range.start.row..=range.end.row {
            let mut row_values: Vec<String> = Vec::with_capacity(width as usize);
            for col in range.start.col..=range.end.col {
                let cell = CellRef { col, row };
                let input = self
                    .engine
                    .cell_input(cell)
                    .map_err(|err| err.to_string())?;
                let value = self
                    .engine
                    .cell_state(cell)
                    .map_err(|err| err.to_string())?
                    .value;
                let format = self
                    .engine
                    .cell_format(cell)
                    .map_err(|err| err.to_string())?;
                row_values.push(format_value(&value, format.decimals));
                cells.push(CopyCell {
                    input,
                    value,
                    format,
                });
            }
            lines.push(row_values.join("\t"));
        }
        let text = lines.join("\n");
        self.last_copy_text = Some(text.clone());
        self.copy_buffer = Some(CopyBuffer {
            width,
            height,
            cells,
            text,
        });
        Ok(())
    }

    fn apply_paste(&mut self, mode: PasteMode) -> Result<(), String> {
        let text = self
            .pending_paste_text
            .as_deref()
            .ok_or_else(|| "clipboard payload missing".to_string())?;
        let parsed = parse_clipboard_text(text);
        let (width, height) = (parsed.width, parsed.height);
        let bounds = self.engine.bounds();
        if self.selected.col + width - 1 > bounds.max_columns
            || self.selected.row + height - 1 > bounds.max_rows
        {
            return Err("paste target exceeds sheet bounds".to_string());
        }

        let source_buffer = self
            .copy_buffer
            .as_ref()
            .filter(|buffer| buffer.text == text)
            .cloned();
        for row_off in 0..height {
            for col_off in 0..width {
                let target = CellRef {
                    col: self.selected.col + col_off,
                    row: self.selected.row + row_off,
                };
                if let Some(reason) = self.editing_block_reason(target) {
                    return Err(reason);
                }

                let idx = (row_off as usize) * (width as usize) + (col_off as usize);
                match mode {
                    PasteMode::All => {
                        if let Some(buffer) = source_buffer.as_ref() {
                            let src = buffer
                                .cell_at(col_off, row_off)
                                .ok_or_else(|| "copy buffer shape mismatch".to_string())?;
                            self.apply_source_input(target, src.input.clone())?;
                            self.engine
                                .set_cell_format(target, src.format.clone())
                                .map_err(|err| err.to_string())?;
                        } else {
                            self.apply_source_input(target, Some(parsed.cells[idx].clone()))?;
                        }
                    }
                    PasteMode::Formulas => {
                        if let Some(buffer) = source_buffer.as_ref() {
                            let src = buffer
                                .cell_at(col_off, row_off)
                                .ok_or_else(|| "copy buffer shape mismatch".to_string())?;
                            self.apply_source_input(target, src.input.clone())?;
                        } else {
                            self.apply_source_input(target, Some(parsed.cells[idx].clone()))?;
                        }
                    }
                    PasteMode::Values | PasteMode::ValuesKeepDestinationFormatting => {
                        if let Some(buffer) = source_buffer.as_ref() {
                            let src = buffer
                                .cell_at(col_off, row_off)
                                .ok_or_else(|| "copy buffer shape mismatch".to_string())?;
                            self.apply_value_input(target, value_to_plain_input(&src.value))?;
                        } else {
                            self.apply_value_input(
                                target,
                                coerce_clipboard_cell_to_value(parsed.cells[idx].clone()),
                            )?;
                        }
                    }
                    PasteMode::Formatting => {
                        let Some(buffer) = source_buffer.as_ref() else {
                            return Err("formatting paste requires copied cells from DNA VisiCalc"
                                .to_string());
                        };
                        let src = buffer
                            .cell_at(col_off, row_off)
                            .ok_or_else(|| "copy buffer shape mismatch".to_string())?;
                        self.engine
                            .set_cell_format(target, src.format.clone())
                            .map_err(|err| err.to_string())?;
                    }
                }
            }
        }
        Ok(())
    }

    fn apply_source_input(
        &mut self,
        target: CellRef,
        input: Option<CellInput>,
    ) -> Result<(), String> {
        match input {
            Some(CellInput::Number(n)) => {
                self.engine.set_number(target, n).map_err(|e| e.to_string())
            }
            Some(CellInput::Text(t)) => self.engine.set_text(target, t).map_err(|e| e.to_string()),
            Some(CellInput::Formula(f)) => self
                .engine
                .set_formula(target, &f)
                .map_err(|e| e.to_string()),
            None => self.engine.clear_cell(target).map_err(|e| e.to_string()),
        }
    }

    fn apply_value_input(&mut self, target: CellRef, input: CellInput) -> Result<(), String> {
        match input {
            CellInput::Number(n) => self.engine.set_number(target, n).map_err(|e| e.to_string()),
            CellInput::Text(t) => self.engine.set_text(target, t).map_err(|e| e.to_string()),
            CellInput::Formula(_) => Err("value paste cannot apply formulas".to_string()),
        }
    }

    fn editing_block_reason(&self, cell: CellRef) -> Option<String> {
        let anchor = self.engine.spill_anchor_for_cell(cell).ok().flatten()?;
        Some(format!(
            "Cannot edit spilled cell {cell}; edit anchor {anchor}"
        ))
    }
}

fn apply_input_to_cell(engine: &mut Engine, cell_ref: &str, input: &str) -> Result<(), String> {
    if input.starts_with('=') || input.starts_with('@') {
        engine
            .set_formula_a1(cell_ref, input)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    if let Ok(number) = input.parse::<f64>() {
        engine
            .set_number_a1(cell_ref, number)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    engine
        .set_text_a1(cell_ref, input.to_string())
        .map_err(|err| err.to_string())
}

fn apply_input_to_name(engine: &mut Engine, name: &str, input: &str) -> Result<(), String> {
    if input.starts_with('=') || input.starts_with('@') {
        engine
            .set_name_formula(name, input)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    if let Ok(number) = input.parse::<f64>() {
        engine
            .set_name_number(name, number)
            .map_err(|err| err.to_string())?;
        return Ok(());
    }

    engine
        .set_name_text(name, input.to_string())
        .map_err(|err| err.to_string())
}

fn parse_on_off(input: &str) -> Option<bool> {
    match input.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" => Some(true),
        "off" | "false" | "0" => Some(false),
        _ => None,
    }
}

impl CopyBuffer {
    fn cell_at(&self, col_off: u16, row_off: u16) -> Option<&CopyCell> {
        if col_off >= self.width || row_off >= self.height {
            return None;
        }
        let idx = (row_off as usize) * (self.width as usize) + (col_off as usize);
        self.cells.get(idx)
    }
}

#[derive(Debug, Clone)]
struct ClipboardGrid {
    width: u16,
    height: u16,
    cells: Vec<CellInput>,
}

fn parse_clipboard_text(text: &str) -> ClipboardGrid {
    let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut lines: Vec<&str> = normalized.split('\n').collect();
    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }
    if lines.is_empty() {
        return ClipboardGrid {
            width: 1,
            height: 1,
            cells: vec![CellInput::Text(String::new())],
        };
    }

    let row_cells: Vec<Vec<CellInput>> = lines
        .iter()
        .map(|line| line.split('\t').map(parse_value_cell).collect::<Vec<_>>())
        .collect();
    let width = row_cells
        .iter()
        .map(|row| row.len())
        .max()
        .unwrap_or(1)
        .max(1) as u16;
    let height = row_cells.len() as u16;
    let mut cells = Vec::with_capacity((width as usize) * (height as usize));
    for row in row_cells {
        for cell in &row {
            cells.push(cell.clone());
        }
        for _ in row.len()..(width as usize) {
            cells.push(CellInput::Text(String::new()));
        }
    }
    ClipboardGrid {
        width,
        height,
        cells,
    }
}

fn parse_value_cell(raw: &str) -> CellInput {
    let trimmed = raw.trim();
    if trimmed.starts_with('=') || trimmed.starts_with('@') {
        return CellInput::Formula(trimmed.to_string());
    }
    if let Ok(n) = trimmed.parse::<f64>() {
        CellInput::Number(n)
    } else {
        CellInput::Text(raw.to_string())
    }
}

fn coerce_clipboard_cell_to_value(input: CellInput) -> CellInput {
    match input {
        CellInput::Formula(formula) => CellInput::Text(formula),
        other => other,
    }
}

fn value_to_plain_input(value: &Value) -> CellInput {
    match value {
        Value::Number(n) => CellInput::Number(*n),
        Value::Text(t) => CellInput::Text(t.clone()),
        Value::Bool(true) => CellInput::Text("TRUE".to_string()),
        Value::Bool(false) => CellInput::Text("FALSE".to_string()),
        Value::Blank => CellInput::Text(String::new()),
        Value::Error(err) => CellInput::Text(format!("#ERR {err}")),
    }
}

#[derive(Debug, Clone)]
pub struct GridSnapshot {
    pub headers: Vec<String>,
    pub rows: Vec<GridRow>,
}

#[derive(Debug, Clone)]
pub struct GridRow {
    pub row_label: u16,
    pub cells: Vec<GridCell>,
}

#[derive(Debug, Clone)]
pub struct GridCell {
    pub active: bool,
    pub in_selection: bool,
    pub value: String,
    pub is_text: bool,
    pub spill_role: SpillRole,
    pub format: CellFormat,
}

fn format_range(range: dnavisicalc_core::CellRange) -> String {
    if range.start == range.end {
        range.start.to_string()
    } else {
        format!("{}:{}", range.start, range.end)
    }
}

pub fn format_value(value: &Value, decimals: Option<u8>) -> String {
    match value {
        Value::Number(n) => {
            if let Some(digits) = decimals {
                format!("{:.*}", digits as usize, n)
            } else {
                let mut s = n.to_string();
                if s.contains('.') {
                    while s.ends_with('0') {
                        s.pop();
                    }
                    if s.ends_with('.') {
                        s.pop();
                    }
                }
                if s.is_empty() { "0".to_string() } else { s }
            }
        }
        Value::Bool(true) => "TRUE".to_string(),
        Value::Bool(false) => "FALSE".to_string(),
        Value::Text(text) => text.clone(),
        Value::Blank => String::new(),
        Value::Error(err) => format!("#ERR {err}"),
    }
}

#[derive(Debug, Default)]
pub struct ScriptRunner {
    pub app: App,
    pub io: crate::io::MemoryWorkbookIo,
    pub outcomes: Vec<CommandOutcome>,
}

impl ScriptRunner {
    pub fn new() -> Self {
        Self {
            app: App::new(),
            io: crate::io::MemoryWorkbookIo::new(),
            outcomes: Vec::new(),
        }
    }

    pub fn run(&mut self, actions: &[Action]) {
        for action in actions {
            let outcome = self.app.apply(action.clone(), &mut self.io);
            self.outcomes.push(outcome.clone());
            if outcome == CommandOutcome::Quit {
                break;
            }
        }
    }

    pub fn files(&self) -> &HashMap<String, String> {
        self.io.files()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnavisicalc_core::{CellInput, PaletteColor};

    fn run_command(app: &mut App, io: &mut crate::io::MemoryWorkbookIo, command: &str) {
        app.apply(Action::StartCommand, io);
        for ch in command.chars() {
            app.apply(Action::InputChar(ch), io);
        }
        app.apply(Action::Submit, io);
    }

    #[test]
    fn script_runner_edits_and_saves() {
        let mut runner = ScriptRunner::new();
        let actions = vec![
            Action::StartEdit,
            Action::InputChar('1'),
            Action::InputChar('2'),
            Action::Submit,
            Action::StartCommand,
            Action::InputChar('w'),
            Action::InputChar(' '),
            Action::InputChar('m'),
            Action::InputChar('e'),
            Action::InputChar('m'),
            Action::InputChar('.'),
            Action::InputChar('d'),
            Action::InputChar('v'),
            Action::InputChar('c'),
            Action::Submit,
        ];
        runner.run(&actions);

        assert_eq!(
            runner
                .app
                .engine()
                .cell_state_a1("A1")
                .expect("query A1")
                .value,
            Value::Number(12.0)
        );
        assert!(runner.files().contains_key("mem.dvc"));
    }

    #[test]
    fn command_mode_can_set_and_recalc() {
        let mut runner = ScriptRunner::new();
        let cmd = "set A1 5";

        let mut actions = vec![Action::StartCommand];
        for ch in cmd.chars() {
            actions.push(Action::InputChar(ch));
        }
        actions.push(Action::Submit);

        runner.run(&actions);
        assert_eq!(
            runner
                .app
                .engine()
                .cell_state_a1("A1")
                .expect("query")
                .value,
            Value::Number(5.0)
        );
    }

    #[test]
    fn viewport_resizes_and_keeps_selection_visible() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();
        app.set_viewport_dimensions(3, 2);

        for _ in 0..6 {
            app.apply(Action::MoveRight, &mut io);
        }
        for _ in 0..4 {
            app.apply(Action::MoveDown, &mut io);
        }

        let selected = app.selected_cell();
        assert!(selected.col >= app.viewport_col);
        assert!(selected.col < app.viewport_col + app.viewport_width);
        assert!(selected.row >= app.viewport_row);
        assert!(selected.row < app.viewport_row + app.viewport_height);

        app.set_viewport_dimensions(1, 1);
        let selected = app.selected_cell();
        assert_eq!(app.viewport_col, selected.col);
        assert_eq!(app.viewport_row, selected.row);
    }

    #[test]
    fn cannot_edit_spilled_child_cell() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(Action::StartCommand, &mut io);
        for ch in "set A1 =SEQUENCE(2)".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);

        app.apply(Action::MoveDown, &mut io);
        app.apply(Action::StartEdit, &mut io);
        assert_eq!(app.mode(), AppMode::Navigate);
        assert!(app.status().contains("Cannot edit spilled cell"));
    }

    #[test]
    fn visible_grid_marks_spill_anchor_and_members() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(Action::StartCommand, &mut io);
        for ch in "set A1 =SEQUENCE(2,2)".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);

        let grid = app.visible_grid(2, 2);
        assert_eq!(grid.rows[0].cells[0].spill_role, SpillRole::Anchor);
        assert_eq!(grid.rows[0].cells[1].spill_role, SpillRole::Member);
        assert_eq!(grid.rows[1].cells[0].spill_role, SpillRole::Member);
        assert_eq!(grid.rows[1].cells[1].spill_role, SpillRole::Member);
    }

    #[test]
    fn status_persists_after_save_and_navigation() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(Action::StartCommand, &mut io);
        for ch in "set A1 1".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);
        app.apply(Action::StartCommand, &mut io);
        for ch in "w demo.dvc".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);

        let saved_status = app.status().to_string();
        app.apply(Action::MoveRight, &mut io);
        app.apply(Action::MoveDown, &mut io);
        assert_eq!(app.status(), saved_status);
    }

    #[test]
    fn toggle_help_changes_visibility() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();
        assert!(!app.help_visible());

        app.apply(Action::ToggleHelp, &mut io);
        assert!(app.help_visible());

        app.apply(Action::ToggleHelp, &mut io);
        assert!(!app.help_visible());
    }

    #[test]
    fn save_state_label_tracks_modifications_and_writes() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();
        assert_eq!(app.save_state_label(), "never saved");

        app.apply(Action::StartEdit, &mut io);
        app.apply(Action::InputChar('1'), &mut io);
        app.apply(Action::Submit, &mut io);
        assert_eq!(app.save_state_label(), "unsaved changes");

        app.apply(Action::StartCommand, &mut io);
        for ch in "w state.dvc".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);
        assert_eq!(app.current_path(), Some("state.dvc"));
        assert_eq!(app.save_state_label(), "saved");
    }

    #[test]
    fn clear_selection_action_clears_multiple_cells() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(Action::StartCommand, &mut io);
        for ch in "set A1 1".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);
        app.apply(Action::StartCommand, &mut io);
        for ch in "set B1 2".chars() {
            app.apply(Action::InputChar(ch), &mut io);
        }
        app.apply(Action::Submit, &mut io);

        app.apply(Action::ExtendRight, &mut io);
        app.apply(Action::ClearSelection, &mut io);

        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Blank
        );
        assert_eq!(
            app.engine().cell_state_a1("B1").expect("B1").value,
            Value::Blank
        );
    }

    #[test]
    fn paste_all_copies_formula_and_format_from_internal_copy_buffer() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 =1+1");
        run_command(&mut app, &mut io, "fmt fg fern");

        app.apply(Action::CopySelection, &mut io);
        let clipboard_text = app
            .last_copy_text()
            .map(ToString::to_string)
            .expect("clipboard text");

        app.apply(Action::MoveRight, &mut io);
        app.apply(Action::BeginPasteFromClipboard(clipboard_text), &mut io);
        app.apply(Action::Submit, &mut io);

        assert_eq!(
            app.engine().cell_input_a1("B1").expect("B1 input"),
            Some(CellInput::Formula("=1+1".to_string()))
        );
        assert_eq!(
            app.engine().cell_format_a1("B1").expect("B1 format").fg,
            Some(PaletteColor::Fern)
        );
    }

    #[test]
    fn paste_values_keep_destination_format_preserves_target_format() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 3.14159");

        app.apply(Action::MoveRight, &mut io);
        run_command(&mut app, &mut io, "fmt bg slate");
        app.apply(Action::MoveLeft, &mut io);

        app.apply(Action::CopySelection, &mut io);
        let clipboard_text = app
            .last_copy_text()
            .map(ToString::to_string)
            .expect("clipboard text");

        app.apply(Action::MoveRight, &mut io);
        app.apply(Action::BeginPasteFromClipboard(clipboard_text), &mut io);
        app.apply(Action::InputChar('4'), &mut io);
        app.apply(Action::Submit, &mut io);

        assert_eq!(
            app.engine().cell_state_a1("B1").expect("B1 value").value,
            Value::Number(3.14159)
        );
        assert_eq!(
            app.engine().cell_format_a1("B1").expect("B1 format").bg,
            Some(PaletteColor::Slate)
        );
    }

    #[test]
    fn external_formula_text_can_be_pasted_in_formula_mode() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(Action::BeginPasteFromClipboard("=1+2".to_string()), &mut io);
        app.apply(Action::InputChar('2'), &mut io);
        app.apply(Action::Submit, &mut io);

        assert_eq!(
            app.engine().cell_input_a1("A1").expect("A1 input"),
            Some(CellInput::Formula("=1+2".to_string()))
        );
        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1 value").value,
            Value::Number(3.0)
        );
    }
}
