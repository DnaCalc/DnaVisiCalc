use std::collections::{HashMap, HashSet};

use crossterm::event::KeyEvent;
use dnavisicalc_engine::{
    CellFormat, CellInput, CellRange, CellRef, Engine, PaletteColor, RecalcMode, Value,
    col_index_to_label,
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
    ToggleChart,
    ToggleControlsFocus,
    TypeChar(char),
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    Slider,
    Checkbox,
    Button,
}

#[derive(Debug, Clone)]
pub struct PanelControl {
    pub name: String,
    pub kind: ControlKind,
    pub value: f64,
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
pub struct ChartState {
    pub source_range: CellRange,
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub range_label: String,
    pub labels: Vec<String>,
    pub series: Vec<ChartSeries>,
}

#[derive(Debug, Clone)]
pub struct ChartSeries {
    pub name: String,
    pub values: Vec<f64>,
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

#[derive(Debug)]
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
    chart_state: Option<ChartState>,
    controls: Vec<PanelControl>,
    controls_focus: usize,
    controls_focused: bool,
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
            chart_state: None,
            controls: Vec::new(),
            controls_focus: 0,
            controls_focused: false,
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

    pub fn selected_range(&self) -> dnavisicalc_engine::CellRange {
        let anchor = self.selection_anchor.unwrap_or(self.selected);
        dnavisicalc_engine::CellRange::new(anchor, self.selected)
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

    pub fn command_hint(&self) -> &'static str {
        let buf = self.command_buffer.trim();
        if buf.is_empty() {
            return "w|o|set|name|fmt|chart|ctrl|insrow|delrow|inscol|delcol|mode|r|q|help";
        }
        let mut parts = buf.split_whitespace();
        let cmd = parts.next().unwrap_or("");
        let arg1 = parts.next();
        let arg2 = parts.next();

        match cmd {
            "w" | "write" => "w [path]",
            "o" | "open" => "o <path>",
            "set" => match arg1 {
                None => "set <cell> <value|formula>",
                Some(_) if arg2.is_none() => "set <cell> <value|formula>",
                _ => "set <cell> <value|formula>",
            },
            "name" => match arg1 {
                None => "name <NAME> <expr> | name clear <NAME>",
                Some(a) if a.eq_ignore_ascii_case("clear") && arg2.is_none() => "name clear <NAME>",
                Some(_) if arg2.is_none() => "name <NAME> <value|formula>",
                _ => "",
            },
            "fmt" => match arg1 {
                None => "fmt decimals|bold|italic|fg|bg|clear",
                Some("decimals") => "fmt decimals <0..9|none>",
                Some("bold") => "fmt bold on|off",
                Some("italic") => "fmt italic on|off",
                Some("fg") => "fmt fg <color|none>",
                Some("bg") => "fmt bg <color|none>",
                Some("clear") => "fmt clear",
                _ => "fmt decimals|bold|italic|fg|bg|clear",
            },
            "chart" => "chart — toggle bar chart from selection",
            "ctrl" => match parts.next() {
                None => "ctrl add slider|checkbox|button <NAME> | ctrl remove <NAME> | ctrl list",
                Some("add") => "ctrl add slider|checkbox|button <NAME>",
                Some("remove") => "ctrl remove <NAME>",
                Some("list") => "ctrl list",
                _ => "ctrl add|remove|list",
            },
            "insrow" | "insertrow" | "ir" => "insrow [at]",
            "delrow" | "deleterow" | "dr" => "delrow [at]",
            "inscol" | "insertcol" | "ic" => "inscol [at]",
            "delcol" | "deletecol" | "dc" => "delcol [at]",
            "mode" => "mode auto|manual",
            "r" | "recalc" => "r — recalculate",
            "q" | "quit" => "q — quit",
            "help" | "?" => "help — show help",
            _ => {
                // Prefix match: show commands that start with what's typed
                let candidates: Vec<&str> = [
                    "w", "o", "set", "name", "fmt", "chart", "ctrl", "insrow", "delrow", "inscol",
                    "delcol", "mode", "r", "q", "help",
                ]
                .into_iter()
                .filter(|c| c.starts_with(cmd))
                .collect();
                if candidates.is_empty() {
                    ""
                } else if candidates.len() == 1 {
                    match candidates[0] {
                        "w" => "w [path]",
                        "o" => "o <path>",
                        "set" => "set <cell> <value|formula>",
                        "name" => "name <NAME> <expr> | name clear <NAME>",
                        "fmt" => "fmt decimals|bold|italic|fg|bg|clear",
                        "chart" => "chart — toggle bar chart",
                        "ctrl" => "ctrl add|remove|list",
                        "insrow" => "insrow [at]",
                        "delrow" => "delrow [at]",
                        "inscol" => "inscol [at]",
                        "delcol" => "delcol [at]",
                        "mode" => "mode auto|manual",
                        _ => "",
                    }
                } else {
                    "w|o|set|name|fmt|chart|ctrl|insrow|delrow|inscol|delcol|mode|r|q|help"
                }
            }
        }
    }

    pub fn chart_state(&self) -> Option<&ChartState> {
        self.chart_state.as_ref()
    }

    pub fn controls(&self) -> &[PanelControl] {
        &self.controls
    }

    pub fn controls_focused(&self) -> bool {
        self.controls_focused
    }

    pub fn controls_focus(&self) -> usize {
        self.controls_focus
    }

    pub fn has_right_panel(&self) -> bool {
        self.chart_state.is_some() || !self.controls.is_empty()
    }

    pub fn chart_data(&self) -> Option<ChartData> {
        let chart = self.chart_state.as_ref()?;
        let range = &chart.source_range;
        let num_cols = range.end.col - range.start.col + 1;
        let num_rows = range.end.row - range.start.row + 1;

        let range_label = if range.start == range.end {
            range.start.to_string()
        } else {
            format!("{}:{}", range.start, range.end)
        };

        let cell_value_f64 = |cell: CellRef| -> f64 {
            self.engine
                .cell_state(cell)
                .ok()
                .and_then(|s| s.value.as_f64())
                .unwrap_or(0.0)
        };

        let cell_value_text = |cell: CellRef| -> String {
            self.engine
                .cell_state(cell)
                .ok()
                .map(|s| match &s.value {
                    Value::Text(t) => t.clone(),
                    Value::Number(n) => format_value(&Value::Number(*n), None),
                    Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                    Value::Blank => String::new(),
                    Value::Error(e) => format!("#ERR {e}"),
                })
                .unwrap_or_default()
        };

        if num_rows == 1 {
            // Single row: each col = a bar
            let labels: Vec<String> = (range.start.col..=range.end.col)
                .map(col_index_to_label)
                .collect();
            let values: Vec<f64> = (range.start.col..=range.end.col)
                .map(|c| {
                    cell_value_f64(CellRef {
                        col: c,
                        row: range.start.row,
                    })
                })
                .collect();
            Some(ChartData {
                range_label,
                labels,
                series: vec![ChartSeries {
                    name: range.start.row.to_string(),
                    values,
                }],
            })
        } else if num_cols == 1 {
            // Single column: each row = a bar
            let labels: Vec<String> = (range.start.row..=range.end.row)
                .map(|r| r.to_string())
                .collect();
            let values: Vec<f64> = (range.start.row..=range.end.row)
                .map(|r| {
                    cell_value_f64(CellRef {
                        col: range.start.col,
                        row: r,
                    })
                })
                .collect();
            Some(ChartData {
                range_label,
                labels,
                series: vec![ChartSeries {
                    name: col_index_to_label(range.start.col),
                    values,
                }],
            })
        } else {
            // Multi-row: first row = labels, remaining rows = value series
            let labels: Vec<String> = (range.start.col..=range.end.col)
                .map(|c| {
                    cell_value_text(CellRef {
                        col: c,
                        row: range.start.row,
                    })
                })
                .collect();
            let series: Vec<ChartSeries> = (range.start.row + 1..=range.end.row)
                .map(|r| {
                    let name = r.to_string();
                    let values: Vec<f64> = (range.start.col..=range.end.col)
                        .map(|c| cell_value_f64(CellRef { col: c, row: r }))
                        .collect();
                    ChartSeries { name, values }
                })
                .collect();
            Some(ChartData {
                range_label,
                labels,
                series,
            })
        }
    }

    pub fn tick_streams(&mut self, elapsed_secs: f64) -> bool {
        self.engine.tick_streams(elapsed_secs)
    }

    pub fn has_stream_cells(&self) -> bool {
        self.engine.has_stream_cells()
    }

    fn recalculate_now(&mut self) {
        match self.engine.recalculate() {
            Ok(()) => self.status = "Recalculated".to_string(),
            Err(err) => self.status = format!("Recalc error: {err}"),
        }
    }

    fn apply_controls_navigate(&mut self, action: Action) -> CommandOutcome {
        match action {
            Action::MoveUp => {
                if !self.controls.is_empty() && self.controls_focus > 0 {
                    self.controls_focus -= 1;
                }
            }
            Action::MoveDown => {
                if !self.controls.is_empty() && self.controls_focus < self.controls.len() - 1 {
                    self.controls_focus += 1;
                }
            }
            Action::MoveLeft => {
                if let Some(ctrl) = self.controls.get(self.controls_focus) {
                    if ctrl.kind == ControlKind::Slider {
                        let new_val = (ctrl.value - 1.0).max(0.0);
                        self.controls[self.controls_focus].value = new_val;
                        self.sync_control_to_engine(self.controls_focus);
                    }
                }
            }
            Action::MoveRight => {
                if let Some(ctrl) = self.controls.get(self.controls_focus) {
                    if ctrl.kind == ControlKind::Slider {
                        let new_val = (ctrl.value + 1.0).min(100.0);
                        self.controls[self.controls_focus].value = new_val;
                        self.sync_control_to_engine(self.controls_focus);
                    }
                }
            }
            Action::ExtendLeft => {
                if let Some(ctrl) = self.controls.get(self.controls_focus) {
                    if ctrl.kind == ControlKind::Slider {
                        let new_val = (ctrl.value - 10.0).max(0.0);
                        self.controls[self.controls_focus].value = new_val;
                        self.sync_control_to_engine(self.controls_focus);
                    }
                }
            }
            Action::ExtendRight => {
                if let Some(ctrl) = self.controls.get(self.controls_focus) {
                    if ctrl.kind == ControlKind::Slider {
                        let new_val = (ctrl.value + 10.0).min(100.0);
                        self.controls[self.controls_focus].value = new_val;
                        self.sync_control_to_engine(self.controls_focus);
                    }
                }
            }
            Action::Submit | Action::TypeChar(' ') => {
                if let Some(ctrl) = self.controls.get(self.controls_focus) {
                    match ctrl.kind {
                        ControlKind::Checkbox => {
                            let new_val = if ctrl.value == 0.0 { 1.0 } else { 0.0 };
                            self.controls[self.controls_focus].value = new_val;
                            self.sync_control_to_engine(self.controls_focus);
                        }
                        ControlKind::Button => {
                            self.controls[self.controls_focus].value += 1.0;
                            self.sync_control_to_engine(self.controls_focus);
                        }
                        ControlKind::Slider => {}
                    }
                }
            }
            Action::Cancel | Action::ToggleControlsFocus => {
                self.controls_focused = false;
                self.status = "Controls unfocused".to_string();
            }
            Action::Recalculate => self.recalculate_now(),
            _ => {}
        }
        CommandOutcome::Continue
    }

    fn sync_control_to_engine(&mut self, index: usize) {
        if let Some(ctrl) = self.controls.get(index) {
            let _ = self.engine.set_name_number(&ctrl.name, ctrl.value);
        }
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
        if self.controls_focused {
            return self.apply_controls_navigate(action);
        }
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
                    "Help shown (press ?/Esc to close)".to_string()
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
            Action::Recalculate => self.recalculate_now(),
            Action::ToggleControlsFocus => {
                if !self.controls.is_empty() {
                    self.controls_focused = true;
                    if self.controls_focus >= self.controls.len() {
                        self.controls_focus = 0;
                    }
                    self.status =
                        "Controls focused (↑↓ navigate, ←→ adjust, Space toggle, Esc back)"
                            .to_string();
                } else {
                    self.status =
                        "No controls defined. Use :ctrl add slider|checkbox|button <NAME>"
                            .to_string();
                }
            }
            Action::ToggleChart => {
                if self.chart_state.is_some() {
                    self.chart_state = None;
                    self.status = "Chart removed".to_string();
                } else {
                    let range = self.selected_range();
                    let label = if range.start == range.end {
                        range.start.to_string()
                    } else {
                        format!("{}:{}", range.start, range.end)
                    };
                    self.chart_state = Some(ChartState {
                        source_range: range,
                    });
                    self.status = format!("Chart: {label}");
                }
            }
            Action::TypeChar(ch) => {
                if let Some(reason) = self.editing_block_reason(self.selected) {
                    self.status = reason;
                    return CommandOutcome::Continue;
                }
                self.mode = AppMode::Edit;
                self.edit_buffer.clear();
                self.edit_buffer.push(ch);
                self.status = format!("Edit {}", self.selected);
            }
            Action::Quit => return CommandOutcome::Quit,
            Action::Cancel => {
                if self.help_visible {
                    self.help_visible = false;
                    self.status = "Help hidden".to_string();
                }
            }
            Action::PasteModeNext
            | Action::PasteModePrev
            | Action::InputChar(_)
            | Action::Backspace
            | Action::Submit => {}
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
            Action::Recalculate => self.recalculate_now(),
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
            | Action::ToggleChart
            | Action::ToggleControlsFocus
            | Action::TypeChar(_)
            | Action::StartEdit
            | Action::StartCommand
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
            Action::Recalculate => self.recalculate_now(),
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
            | Action::ToggleChart
            | Action::ToggleControlsFocus
            | Action::TypeChar(_)
            | Action::StartEdit
            | Action::StartCommand
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
            Action::Recalculate => self.recalculate_now(),
            Action::CopySelection
            | Action::PasteFromClipboard
            | Action::BeginPasteFromClipboard(_)
            | Action::ToggleHelp
            | Action::ToggleChart
            | Action::ToggleControlsFocus
            | Action::TypeChar(_)
            | Action::StartEdit
            | Action::StartCommand
            | Action::Backspace
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
            "r" | "recalc" => self.recalculate_now(),
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
            "chart" => {
                if self.chart_state.is_some() {
                    self.chart_state = None;
                    self.status = "Chart removed".to_string();
                } else {
                    let range = self.selected_range();
                    let label = if range.start == range.end {
                        range.start.to_string()
                    } else {
                        format!("{}:{}", range.start, range.end)
                    };
                    self.chart_state = Some(ChartState {
                        source_range: range,
                    });
                    self.status = format!("Chart: {label}");
                }
            }
            "ctrl" => {
                let Some(sub) = parts.next() else {
                    self.status = "Usage: ctrl add slider|checkbox|button <NAME> | ctrl remove <NAME> | ctrl list".to_string();
                    return CommandOutcome::Continue;
                };
                match sub {
                    "add" => {
                        let Some(kind_str) = parts.next() else {
                            self.status =
                                "Usage: ctrl add slider|checkbox|button <NAME>".to_string();
                            return CommandOutcome::Continue;
                        };
                        let kind = match kind_str {
                            "slider" => ControlKind::Slider,
                            "checkbox" => ControlKind::Checkbox,
                            "button" => ControlKind::Button,
                            _ => {
                                self.status =
                                    "Usage: ctrl add slider|checkbox|button <NAME>".to_string();
                                return CommandOutcome::Continue;
                            }
                        };
                        let Some(name) = parts.next() else {
                            self.status =
                                "Usage: ctrl add slider|checkbox|button <NAME>".to_string();
                            return CommandOutcome::Continue;
                        };
                        let name_upper = name.to_ascii_uppercase();
                        if self.controls.iter().any(|c| c.name == name_upper) {
                            self.status = format!("Control {name_upper} already exists");
                            return CommandOutcome::Continue;
                        }
                        if self.controls.len() >= 6 {
                            self.status = "Maximum 6 controls allowed".to_string();
                            return CommandOutcome::Continue;
                        }
                        let initial_value = match kind {
                            ControlKind::Slider => 50.0,
                            ControlKind::Checkbox => 0.0,
                            ControlKind::Button => 0.0,
                        };
                        self.controls.push(PanelControl {
                            name: name_upper.clone(),
                            kind,
                            value: initial_value,
                        });
                        let idx = self.controls.len() - 1;
                        self.sync_control_to_engine(idx);
                        self.status = format!("Added {kind_str} control {name_upper}");
                    }
                    "remove" => {
                        let Some(name) = parts.next() else {
                            self.status = "Usage: ctrl remove <NAME>".to_string();
                            return CommandOutcome::Continue;
                        };
                        let name_upper = name.to_ascii_uppercase();
                        if let Some(pos) = self.controls.iter().position(|c| c.name == name_upper) {
                            self.controls.remove(pos);
                            let _ = self.engine.clear_name(&name_upper);
                            if self.controls_focus >= self.controls.len()
                                && !self.controls.is_empty()
                            {
                                self.controls_focus = self.controls.len() - 1;
                            }
                            if self.controls.is_empty() {
                                self.controls_focused = false;
                            }
                            self.status = format!("Removed control {name_upper}");
                        } else {
                            self.status = format!("No control named {name_upper}");
                        }
                    }
                    "list" => {
                        if self.controls.is_empty() {
                            self.status = "No controls defined".to_string();
                        } else {
                            let list: Vec<String> = self
                                .controls
                                .iter()
                                .map(|c| {
                                    let kind_str = match c.kind {
                                        ControlKind::Slider => "slider",
                                        ControlKind::Checkbox => "checkbox",
                                        ControlKind::Button => "button",
                                    };
                                    format!("{}({}={}", c.name, kind_str, c.value)
                                })
                                .collect();
                            self.status = format!("Controls: {}", list.join(", "));
                        }
                    }
                    _ => {
                        self.status = "Usage: ctrl add slider|checkbox|button <NAME> | ctrl remove <NAME> | ctrl list".to_string();
                    }
                }
            }
            "insrow" | "insertrow" | "ir" => {
                let Some(at) = parse_optional_u16(parts.next(), self.selected.row) else {
                    self.status = "Usage: insrow [at]".to_string();
                    return CommandOutcome::Continue;
                };
                if parts.next().is_some() {
                    self.status = "Usage: insrow [at]".to_string();
                    return CommandOutcome::Continue;
                }
                match self.engine.insert_row(at) {
                    Ok(()) => self.status = format!("Inserted row {at}"),
                    Err(err) => self.status = format!("Insert row error: {err}"),
                }
            }
            "delrow" | "deleterow" | "dr" => {
                let Some(at) = parse_optional_u16(parts.next(), self.selected.row) else {
                    self.status = "Usage: delrow [at]".to_string();
                    return CommandOutcome::Continue;
                };
                if parts.next().is_some() {
                    self.status = "Usage: delrow [at]".to_string();
                    return CommandOutcome::Continue;
                }
                match self.engine.delete_row(at) {
                    Ok(()) => self.status = format!("Deleted row {at}"),
                    Err(err) => self.status = format!("Delete row error: {err}"),
                }
            }
            "inscol" | "insertcol" | "ic" => {
                let Some(at) = parse_optional_u16(parts.next(), self.selected.col) else {
                    self.status = "Usage: inscol [at]".to_string();
                    return CommandOutcome::Continue;
                };
                if parts.next().is_some() {
                    self.status = "Usage: inscol [at]".to_string();
                    return CommandOutcome::Continue;
                }
                match self.engine.insert_col(at) {
                    Ok(()) => self.status = format!("Inserted col {at}"),
                    Err(err) => self.status = format!("Insert col error: {err}"),
                }
            }
            "delcol" | "deletecol" | "dc" => {
                let Some(at) = parse_optional_u16(parts.next(), self.selected.col) else {
                    self.status = "Usage: delcol [at]".to_string();
                    return CommandOutcome::Continue;
                };
                if parts.next().is_some() {
                    self.status = "Usage: delcol [at]".to_string();
                    return CommandOutcome::Continue;
                }
                match self.engine.delete_col(at) {
                    Ok(()) => self.status = format!("Deleted col {at}"),
                    Err(err) => self.status = format!("Delete col error: {err}"),
                }
            }
            "help" | "?" => {
                self.help_visible = true;
                self.status = "Help shown (press ?/Esc to close)".to_string();
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

        // Normalize line endings for comparison: the system clipboard on Windows
        // may return \r\n even though our copy buffer uses \n.
        let normalized_text = text.replace("\r\n", "\n").replace('\r', "\n");
        let source_buffer = self
            .copy_buffer
            .as_ref()
            .filter(|buffer| buffer.text == normalized_text)
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

fn parse_optional_u16(input: Option<&str>, default: u16) -> Option<u16> {
    match input {
        Some(raw) => raw.parse::<u16>().ok(),
        None => Some(default),
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

fn format_range(range: dnavisicalc_engine::CellRange) -> String {
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
        self.run_actions(actions);
    }

    pub fn run_actions(&mut self, actions: &[Action]) {
        for action in actions {
            if self.apply_action(action.clone()) == CommandOutcome::Quit {
                break;
            }
        }
    }

    pub fn run_keys(&mut self, keys: &[KeyEvent]) {
        for key in keys {
            let mode = self.app.mode();
            if let Some(action) = crate::action_from_key(mode, key.clone())
                && self.apply_action(action) == CommandOutcome::Quit
            {
                break;
            }
        }
    }

    fn apply_action(&mut self, action: Action) -> CommandOutcome {
        let outcome = self.app.apply(action, &mut self.io);
        self.outcomes.push(outcome.clone());
        outcome
    }

    pub fn files(&self) -> &HashMap<String, String> {
        self.io.files()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
    use dnavisicalc_engine::{CellInput, PaletteColor};

    fn run_command(app: &mut App, io: &mut crate::io::MemoryWorkbookIo, command: &str) {
        app.apply(Action::StartCommand, io);
        for ch in command.chars() {
            app.apply(Action::InputChar(ch), io);
        }
        app.apply(Action::Submit, io);
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn append_text_keys(keys: &mut Vec<KeyEvent>, text: &str) {
        for ch in text.chars() {
            keys.push(key(KeyCode::Char(ch)));
        }
    }

    #[test]
    fn script_runner_edits_and_saves() {
        let mut runner = ScriptRunner::new();
        let mut keys = Vec::new();
        keys.push(key(KeyCode::Char('1')));
        keys.push(key(KeyCode::Char('2')));
        keys.push(key(KeyCode::Enter));
        keys.push(key(KeyCode::Char(':')));
        append_text_keys(&mut keys, "w mem.dvc");
        keys.push(key(KeyCode::Enter));
        runner.run_keys(&keys);

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
        let mut keys = vec![key(KeyCode::Char(':'))];
        append_text_keys(&mut keys, "set A1 5");
        keys.push(key(KeyCode::Enter));
        runner.run_keys(&keys);
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
    fn recalculate_action_runs_in_command_mode() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 =RAND()");
        let before = app.engine().cell_state_a1("A1").expect("before").value;

        app.apply(Action::StartCommand, &mut io);
        app.apply(Action::InputChar('x'), &mut io);
        app.apply(Action::Recalculate, &mut io);

        assert_eq!(app.mode(), AppMode::Command);
        let after = app.engine().cell_state_a1("A1").expect("after").value;
        assert_ne!(before, after);
        assert_eq!(app.status(), "Recalculated");
        assert_eq!(app.command_buffer(), "x");
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

    #[test]
    fn paste_2x2_external_grid() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        app.apply(
            Action::BeginPasteFromClipboard("1\t2\n3\t4".to_string()),
            &mut io,
        );
        app.apply(Action::Submit, &mut io);

        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Number(1.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("B1").expect("B1").value,
            Value::Number(2.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("A2").expect("A2").value,
            Value::Number(3.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("B2").expect("B2").value,
            Value::Number(4.0)
        );
    }

    #[test]
    fn paste_internal_2x2_copy_buffer() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        // Set up a 2x2 range
        run_command(&mut app, &mut io, "set A1 10");
        run_command(&mut app, &mut io, "set B1 20");
        run_command(&mut app, &mut io, "set A2 30");
        run_command(&mut app, &mut io, "set B2 40");

        // Select A1:B2
        app.apply(Action::ExtendRight, &mut io);
        app.apply(Action::ExtendDown, &mut io);

        // Copy
        app.apply(Action::CopySelection, &mut io);
        let clipboard_text = app
            .last_copy_text()
            .map(ToString::to_string)
            .expect("clipboard text");

        // Navigate to C1 (selected is at B2 after ExtendDown, MoveRight clears anchor)
        app.apply(Action::MoveUp, &mut io); // B1 (deselects anchor)
        app.apply(Action::MoveRight, &mut io); // C1

        // Paste
        app.apply(Action::BeginPasteFromClipboard(clipboard_text), &mut io);
        app.apply(Action::Submit, &mut io);

        assert_eq!(
            app.engine().cell_state_a1("C1").expect("C1").value,
            Value::Number(10.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("D1").expect("D1").value,
            Value::Number(20.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("C2").expect("C2").value,
            Value::Number(30.0)
        );
        assert_eq!(
            app.engine().cell_state_a1("D2").expect("D2").value,
            Value::Number(40.0)
        );
    }

    #[test]
    fn paste_internal_2x2_with_crlf_clipboard() {
        // Windows clipboard returns \r\n but our copy buffer stores \n.
        // The paste logic should still match the internal buffer.
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 =1+1");
        run_command(&mut app, &mut io, "set B1 =2+2");
        run_command(&mut app, &mut io, "set A2 =3+3");
        run_command(&mut app, &mut io, "set B2 =4+4");

        // Select A1:B2 and copy
        app.apply(Action::ExtendRight, &mut io);
        app.apply(Action::ExtendDown, &mut io);
        app.apply(Action::CopySelection, &mut io);

        let original_text = app
            .last_copy_text()
            .map(ToString::to_string)
            .expect("clipboard text");
        assert!(original_text.contains('\n'));
        assert!(!original_text.contains('\r'));

        // Simulate Windows clipboard returning \r\n
        let crlf_text = original_text.replace('\n', "\r\n");

        // Navigate to C1
        app.apply(Action::MoveUp, &mut io);
        app.apply(Action::MoveRight, &mut io);

        // Paste with CRLF text
        app.apply(Action::BeginPasteFromClipboard(crlf_text), &mut io);
        app.apply(Action::Submit, &mut io);

        // Should have formulas, not just values
        assert_eq!(
            app.engine().cell_input_a1("C1").expect("C1"),
            Some(CellInput::Formula("=1+1".to_string()))
        );
        assert_eq!(
            app.engine().cell_input_a1("D1").expect("D1"),
            Some(CellInput::Formula("=2+2".to_string()))
        );
        assert_eq!(
            app.engine().cell_input_a1("C2").expect("C2"),
            Some(CellInput::Formula("=3+3".to_string()))
        );
        assert_eq!(
            app.engine().cell_input_a1("D2").expect("D2"),
            Some(CellInput::Formula("=4+4".to_string()))
        );
    }

    #[test]
    fn esc_dismisses_help_popup() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();
        assert!(!app.help_visible());

        app.apply(Action::ToggleHelp, &mut io);
        assert!(app.help_visible());

        app.apply(Action::Cancel, &mut io);
        assert!(!app.help_visible());
    }

    #[test]
    fn esc_is_noop_when_help_not_visible() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();
        assert!(!app.help_visible());

        app.apply(Action::Cancel, &mut io);
        assert!(!app.help_visible());
        assert_eq!(app.mode(), AppMode::Navigate);
    }

    #[test]
    fn toggle_chart_creates_and_removes() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        // Select A1:A3
        app.apply(Action::ExtendDown, &mut io);
        app.apply(Action::ExtendDown, &mut io);

        app.apply(Action::ToggleChart, &mut io);
        assert!(app.chart_state().is_some());
        let cs = app.chart_state().unwrap();
        assert_eq!(cs.source_range.start, CellRef::from_a1("A1").unwrap());
        assert_eq!(cs.source_range.end, CellRef::from_a1("A3").unwrap());
        assert!(app.status().contains("Chart:"));

        app.apply(Action::ToggleChart, &mut io);
        assert!(app.chart_state().is_none());
        assert!(app.status().contains("Chart removed"));
    }

    #[test]
    fn chart_data_returns_values_from_source_range() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 10");
        run_command(&mut app, &mut io, "set A2 20");
        run_command(&mut app, &mut io, "set A3 30");

        // Select A1:A3
        app.apply(Action::ExtendDown, &mut io);
        app.apply(Action::ExtendDown, &mut io);
        app.apply(Action::ToggleChart, &mut io);

        let data = app.chart_data().expect("chart data should exist");
        assert_eq!(data.labels, vec!["1", "2", "3"]);
        assert_eq!(data.series.len(), 1);
        assert_eq!(data.series[0].values, vec![10.0, 20.0, 30.0]);
    }

    #[test]
    fn type_char_in_navigate_starts_edit_with_fresh_buffer() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        // Pre-populate A1
        run_command(&mut app, &mut io, "set A1 99");
        assert_eq!(app.mode(), AppMode::Navigate);

        // Type '5' in Navigate mode → enters Edit with just "5", not "995"
        app.apply(Action::TypeChar('5'), &mut io);
        assert_eq!(app.mode(), AppMode::Edit);
        assert_eq!(app.edit_buffer(), "5");

        // Continue typing and submit
        app.apply(Action::InputChar('0'), &mut io);
        app.apply(Action::Submit, &mut io);
        assert_eq!(app.mode(), AppMode::Navigate);
        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Number(50.0)
        );
    }

    #[test]
    fn type_char_on_spilled_cell_is_blocked() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 =SEQUENCE(2)");
        app.apply(Action::MoveDown, &mut io);

        app.apply(Action::TypeChar('1'), &mut io);
        assert_eq!(app.mode(), AppMode::Navigate);
        assert!(app.status().contains("Cannot edit spilled cell"));
    }

    #[test]
    fn esc_in_edit_discards_changes() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "set A1 42");
        app.apply(Action::TypeChar('9'), &mut io);
        app.apply(Action::InputChar('9'), &mut io);
        assert_eq!(app.edit_buffer(), "99");

        app.apply(Action::Cancel, &mut io);
        assert_eq!(app.mode(), AppMode::Navigate);
        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Number(42.0)
        );
    }

    #[test]
    fn command_hint_changes_with_input() {
        let app = App::new();
        assert!(app.command_hint().contains("w"));
        assert!(app.command_hint().contains("fmt"));
    }

    #[test]
    fn ctrl_add_creates_control_and_sets_engine_name() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add slider RATE");
        assert_eq!(app.controls().len(), 1);
        assert_eq!(app.controls()[0].name, "RATE");
        assert_eq!(app.controls()[0].kind, ControlKind::Slider);
        assert_eq!(app.controls()[0].value, 50.0);

        // Engine name should be set — verify via a formula referencing it
        run_command(&mut app, &mut io, "set A1 =RATE");
        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Number(50.0)
        );
    }

    #[test]
    fn ctrl_add_checkbox_and_button() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add checkbox ENABLED");
        assert_eq!(app.controls()[0].kind, ControlKind::Checkbox);
        assert_eq!(app.controls()[0].value, 0.0);

        run_command(&mut app, &mut io, "ctrl add button GO");
        assert_eq!(app.controls()[1].kind, ControlKind::Button);
        assert_eq!(app.controls()[1].value, 0.0);
    }

    #[test]
    fn ctrl_remove_clears_control_and_name() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add slider RATE");
        assert_eq!(app.controls().len(), 1);

        run_command(&mut app, &mut io, "ctrl remove RATE");
        assert!(app.controls().is_empty());
        // Name should be cleared — formula referencing it should error
        run_command(&mut app, &mut io, "set A1 =RATE");
        let val = app.engine().cell_state_a1("A1").expect("A1").value;
        assert!(matches!(val, Value::Error(_)));
    }

    #[test]
    fn controls_navigate_adjusts_slider() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add slider RATE");
        // Set up a formula that references RATE before focusing controls
        run_command(&mut app, &mut io, "set A1 =RATE");

        app.apply(Action::ToggleControlsFocus, &mut io);
        assert!(app.controls_focused());

        // Right arrow increments by 1
        app.apply(Action::MoveRight, &mut io);
        assert_eq!(app.controls()[0].value, 51.0);

        // Left arrow decrements by 1
        app.apply(Action::MoveLeft, &mut io);
        assert_eq!(app.controls()[0].value, 50.0);

        // Shift+Right increments by 10
        app.apply(Action::ExtendRight, &mut io);
        assert_eq!(app.controls()[0].value, 60.0);

        // Shift+Left decrements by 10
        app.apply(Action::ExtendLeft, &mut io);
        assert_eq!(app.controls()[0].value, 50.0);

        // Engine name should reflect
        assert_eq!(
            app.engine().cell_state_a1("A1").expect("A1").value,
            Value::Number(50.0)
        );
    }

    #[test]
    fn checkbox_toggle_via_activate() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add checkbox ENABLED");
        app.apply(Action::ToggleControlsFocus, &mut io);
        assert!(app.controls_focused());

        assert_eq!(app.controls()[0].value, 0.0);
        app.apply(Action::TypeChar(' '), &mut io);
        assert_eq!(app.controls()[0].value, 1.0);
        app.apply(Action::TypeChar(' '), &mut io);
        assert_eq!(app.controls()[0].value, 0.0);
    }

    #[test]
    fn button_press_increments_count() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        run_command(&mut app, &mut io, "ctrl add button GO");
        app.apply(Action::ToggleControlsFocus, &mut io);

        app.apply(Action::Submit, &mut io);
        assert_eq!(app.controls()[0].value, 1.0);
        app.apply(Action::Submit, &mut io);
        assert_eq!(app.controls()[0].value, 2.0);
    }

    #[test]
    fn toggle_controls_focus() {
        let mut app = App::new();
        let mut io = crate::io::MemoryWorkbookIo::new();

        // No controls: F3 does nothing
        app.apply(Action::ToggleControlsFocus, &mut io);
        assert!(!app.controls_focused());

        run_command(&mut app, &mut io, "ctrl add slider X");
        app.apply(Action::ToggleControlsFocus, &mut io);
        assert!(app.controls_focused());

        // Esc exits focus
        app.apply(Action::Cancel, &mut io);
        assert!(!app.controls_focused());
    }
}
