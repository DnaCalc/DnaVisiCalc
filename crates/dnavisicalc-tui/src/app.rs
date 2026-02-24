use std::collections::HashMap;

use dnavisicalc_core::{CellInput, CellRef, Engine, RecalcMode, Value, col_index_to_label};

use crate::io::WorkbookIo;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppMode {
    Navigate,
    Edit,
    Command,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
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

#[derive(Debug, Clone)]
pub struct App {
    engine: Engine,
    mode: AppMode,
    selected: CellRef,
    viewport_col: u16,
    viewport_row: u16,
    viewport_width: u16,
    viewport_height: u16,
    edit_buffer: String,
    command_buffer: String,
    status: String,
    last_path: Option<String>,
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
            viewport_col: 1,
            viewport_row: 1,
            viewport_width: 8,
            viewport_height: 12,
            edit_buffer: String::new(),
            command_buffer: String::new(),
            status: "Ready".to_string(),
            last_path: None,
        }
    }

    pub fn from_engine(engine: Engine) -> Self {
        let mut app = Self::new();
        app.engine = engine;
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

    pub fn status(&self) -> &str {
        &self.status
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

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = status.into();
    }

    pub fn apply(&mut self, action: Action, io: &mut dyn WorkbookIo) -> CommandOutcome {
        match self.mode {
            AppMode::Navigate => self.apply_navigate(action, io),
            AppMode::Edit => self.apply_edit(action),
            AppMode::Command => self.apply_command(action, io),
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
                let selected = cell == self.selected;
                let value = self
                    .engine
                    .cell_state(cell)
                    .map(|state| format_value(&state.value))
                    .unwrap_or_else(|_| "#ADDR".to_string());
                let spill_role = match self.engine.spill_range_for_cell(cell).ok().flatten() {
                    Some(range) if range.start == cell => SpillRole::Anchor,
                    Some(_) => SpillRole::Member,
                    None => SpillRole::None,
                };
                cells.push(GridCell {
                    selected,
                    value,
                    spill_role,
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
        self.engine
            .cell_state(self.selected)
            .map(|state| format_value(&state.value))
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
            Ok(None) => String::new(),
            Err(_) => String::new(),
        }
    }

    fn apply_navigate(&mut self, action: Action, io: &mut dyn WorkbookIo) -> CommandOutcome {
        match action {
            Action::MoveLeft => self.move_selection(-1, 0),
            Action::MoveRight => self.move_selection(1, 0),
            Action::MoveUp => self.move_selection(0, -1),
            Action::MoveDown => self.move_selection(0, 1),
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
            Action::InputChar(_) | Action::Backspace | Action::Submit | Action::Cancel => {}
        }

        self.ensure_visible();
        if matches!(
            action,
            Action::MoveLeft | Action::MoveRight | Action::MoveUp | Action::MoveDown
        ) {
            self.status = format!(
                "{} = {}",
                self.selected,
                self.evaluate_display_for_selected()
            );
        }
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
            | Action::StartEdit
            | Action::StartCommand
            | Action::Recalculate
            | Action::Quit => {}
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
            "help" => {
                self.status = "Commands: q, r, mode auto|manual, w <path>, o <path>, set <A1> <value|formula>".to_string();
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

    fn move_selection(&mut self, dx: i16, dy: i16) {
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

    Err("text cells are not supported yet; use numbers or formulas".to_string())
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
    pub selected: bool,
    pub value: String,
    pub spill_role: SpillRole,
}

fn format_range(range: dnavisicalc_core::CellRange) -> String {
    if range.start == range.end {
        range.start.to_string()
    } else {
        format!("{}:{}", range.start, range.end)
    }
}

pub fn format_value(value: &Value) -> String {
    match value {
        Value::Number(n) => {
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
        Value::Bool(true) => "TRUE".to_string(),
        Value::Bool(false) => "FALSE".to_string(),
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
}
