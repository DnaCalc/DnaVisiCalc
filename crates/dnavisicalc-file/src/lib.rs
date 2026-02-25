use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use dnavisicalc_core::{CellInput, CellRef, Engine, EngineError, NameInput, RecalcMode};
use thiserror::Error;

const MAGIC: &str = "DVISICALC\t1";

#[derive(Debug, Error)]
pub enum FileError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error at line {line}: {message}")]
    Parse { line: usize, message: String },
    #[error("engine error while loading: {0}")]
    Engine(#[from] EngineError),
}

pub fn save_to_string(engine: &Engine) -> Result<String, FileError> {
    let mut out = String::new();
    out.push_str(MAGIC);
    out.push('\n');
    out.push_str(match engine.recalc_mode() {
        RecalcMode::Automatic => "MODE\tAUTO\n",
        RecalcMode::Manual => "MODE\tMANUAL\n",
    });

    for (cell, input) in engine.all_cell_inputs() {
        match input {
            CellInput::Number(n) => {
                out.push_str("CELL\t");
                out.push_str(&cell.to_string());
                out.push_str("\tN\t");
                out.push_str(&n.to_string());
                out.push('\n');
            }
            CellInput::Text(text) => {
                out.push_str("CELL\t");
                out.push_str(&cell.to_string());
                out.push_str("\tT\t");
                out.push_str(&escape_field(&text));
                out.push('\n');
            }
            CellInput::Formula(formula) => {
                out.push_str("CELL\t");
                out.push_str(&cell.to_string());
                out.push_str("\tF\t");
                out.push_str(&escape_field(&formula));
                out.push('\n');
            }
        }
    }

    for (name, input) in engine.all_name_inputs() {
        match input {
            NameInput::Number(n) => {
                out.push_str("NAME\t");
                out.push_str(&name);
                out.push_str("\tN\t");
                out.push_str(&n.to_string());
                out.push('\n');
            }
            NameInput::Text(text) => {
                out.push_str("NAME\t");
                out.push_str(&name);
                out.push_str("\tT\t");
                out.push_str(&escape_field(&text));
                out.push('\n');
            }
            NameInput::Formula(formula) => {
                out.push_str("NAME\t");
                out.push_str(&name);
                out.push_str("\tF\t");
                out.push_str(&escape_field(&formula));
                out.push('\n');
            }
        }
    }

    Ok(out)
}

pub fn save_to_path(engine: &Engine, path: impl AsRef<Path>) -> Result<(), FileError> {
    let content = save_to_string(engine)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_from_path(path: impl AsRef<Path>) -> Result<Engine, FileError> {
    let content = fs::read_to_string(path)?;
    load_from_str(&content)
}

pub fn load_from_str(input: &str) -> Result<Engine, FileError> {
    let mut found_header = false;
    let mut mode = RecalcMode::Automatic;
    let mut seen_mode = false;
    let mut seen_cells = BTreeSet::new();
    let mut seen_names = BTreeSet::new();
    let mut entries: Vec<(usize, CellRef, CellInput)> = Vec::new();
    let mut name_entries: Vec<(usize, String, NameInput)> = Vec::new();

    for (idx, raw_line) in input.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if !found_header {
            let header_line = line.strip_prefix('\u{feff}').unwrap_or(line);
            if header_line != MAGIC {
                return Err(FileError::Parse {
                    line: line_no,
                    message: format!("expected header '{MAGIC}'"),
                });
            }
            found_header = true;
            continue;
        }

        let mut parts = raw_line.split('\t');
        let Some(kind) = parts.next() else {
            continue;
        };

        match kind {
            "MODE" => {
                if seen_mode {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "duplicate MODE record".to_string(),
                    });
                }
                let Some(mode_part) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "MODE record missing value".to_string(),
                    });
                };
                if parts.next().is_some() {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "MODE record has extra fields".to_string(),
                    });
                }
                mode = match mode_part.trim() {
                    "AUTO" => RecalcMode::Automatic,
                    "MANUAL" => RecalcMode::Manual,
                    other => {
                        return Err(FileError::Parse {
                            line: line_no,
                            message: format!("unknown mode '{other}'"),
                        });
                    }
                };
                seen_mode = true;
            }
            "CELL" => {
                let Some(addr) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "CELL record missing address".to_string(),
                    });
                };
                let Some(cell_type) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "CELL record missing type".to_string(),
                    });
                };
                let Some(value) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "CELL record missing value".to_string(),
                    });
                };
                if parts.next().is_some() {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "CELL record has extra fields".to_string(),
                    });
                }

                let cell = CellRef::from_a1(addr).map_err(|err| FileError::Parse {
                    line: line_no,
                    message: format!("invalid address '{addr}': {err}"),
                })?;

                if !seen_cells.insert(cell) {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: format!("duplicate cell record for {cell}"),
                    });
                }

                let input = match cell_type.trim() {
                    "N" => {
                        let number = value.trim().parse::<f64>().map_err(|_| FileError::Parse {
                            line: line_no,
                            message: format!("invalid number '{value}'"),
                        })?;
                        CellInput::Number(number)
                    }
                    "T" => {
                        let text = unescape_field(value).map_err(|message| FileError::Parse {
                            line: line_no,
                            message,
                        })?;
                        CellInput::Text(text)
                    }
                    "F" => {
                        let formula =
                            unescape_field(value).map_err(|message| FileError::Parse {
                                line: line_no,
                                message,
                            })?;
                        CellInput::Formula(formula)
                    }
                    other => {
                        return Err(FileError::Parse {
                            line: line_no,
                            message: format!("unknown CELL type '{other}'"),
                        });
                    }
                };

                entries.push((line_no, cell, input));
            }
            "NAME" => {
                let Some(raw_name) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "NAME record missing identifier".to_string(),
                    });
                };
                let Some(name_type) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "NAME record missing type".to_string(),
                    });
                };
                let Some(value) = parts.next() else {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "NAME record missing value".to_string(),
                    });
                };
                if parts.next().is_some() {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: "NAME record has extra fields".to_string(),
                    });
                }

                let name = raw_name.trim().to_ascii_uppercase();
                if !seen_names.insert(name.clone()) {
                    return Err(FileError::Parse {
                        line: line_no,
                        message: format!("duplicate name record for {name}"),
                    });
                }

                let input = match name_type.trim() {
                    "N" => {
                        let number = value.trim().parse::<f64>().map_err(|_| FileError::Parse {
                            line: line_no,
                            message: format!("invalid number '{value}'"),
                        })?;
                        NameInput::Number(number)
                    }
                    "T" => {
                        let text = unescape_field(value).map_err(|message| FileError::Parse {
                            line: line_no,
                            message,
                        })?;
                        NameInput::Text(text)
                    }
                    "F" => {
                        let formula =
                            unescape_field(value).map_err(|message| FileError::Parse {
                                line: line_no,
                                message,
                            })?;
                        NameInput::Formula(formula)
                    }
                    other => {
                        return Err(FileError::Parse {
                            line: line_no,
                            message: format!("unknown NAME type '{other}'"),
                        });
                    }
                };

                name_entries.push((line_no, name, input));
            }
            other => {
                return Err(FileError::Parse {
                    line: line_no,
                    message: format!("unknown record kind '{other}'"),
                });
            }
        }
    }

    if !found_header {
        return Err(FileError::Parse {
            line: 1,
            message: format!("missing header '{MAGIC}'"),
        });
    }

    let mut engine = Engine::new();
    engine.set_recalc_mode(RecalcMode::Manual);
    for (line_no, cell, input) in entries {
        if let Err(err) = engine.set_cell_input(cell, input) {
            return Err(FileError::Parse {
                line: line_no,
                message: format!("failed to apply cell {cell}: {err}"),
            });
        }
    }
    for (line_no, name, input) in name_entries {
        if let Err(err) = engine.set_name_input(&name, input) {
            return Err(FileError::Parse {
                line: line_no,
                message: format!("failed to apply name {name}: {err}"),
            });
        }
    }
    if let Err(err) = engine.recalculate() {
        return Err(FileError::Parse {
            line: 1,
            message: format!("recalculation failed after load: {err}"),
        });
    }
    engine.set_recalc_mode(mode);
    Ok(engine)
}

fn escape_field(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out
}

fn unescape_field(input: &str) -> Result<String, String> {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }

        let Some(next) = chars.next() else {
            return Err("trailing escape in field".to_string());
        };
        match next {
            '\\' => out.push('\\'),
            't' => out.push('\t'),
            'n' => out.push('\n'),
            'r' => out.push('\r'),
            other => return Err(format!("unsupported escape sequence '\\{other}'")),
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dnavisicalc_core::{NameInput, Value};

    #[test]
    fn roundtrips_engine_document() {
        let mut engine = Engine::new();
        engine.set_number_a1("A1", 12.5).expect("set A1");
        engine.set_text_a1("A2", "hello").expect("set A2");
        engine.set_name_number("RATE", 0.2).expect("set RATE");
        engine
            .set_name_formula("BONUS", "=A1*RATE")
            .expect("set BONUS");
        engine
            .set_formula_a1("B1", "@SUM(A1...A1)")
            .expect("set B1 formula");
        engine
            .set_formula_a1("B2", "=A2&\" world\"")
            .expect("set B2 formula");
        engine
            .set_formula_a1("C1", "=BONUS")
            .expect("set C1 formula");

        let text = save_to_string(&engine).expect("serialize engine");
        let loaded = load_from_str(&text).expect("deserialize engine");

        assert_eq!(
            loaded.cell_state_a1("A1").expect("query").value,
            Value::Number(12.5)
        );
        assert_eq!(
            loaded.cell_state_a1("B1").expect("query").value,
            Value::Number(12.5)
        );
        assert_eq!(
            loaded.cell_state_a1("A2").expect("query").value,
            Value::Text("hello".to_string())
        );
        assert_eq!(
            loaded.cell_state_a1("B2").expect("query").value,
            Value::Text("hello world".to_string())
        );
        assert_eq!(
            loaded.cell_state_a1("C1").expect("query").value,
            Value::Number(2.5)
        );
        assert_eq!(
            loaded.name_input("RATE").expect("query RATE"),
            Some(NameInput::Number(0.2))
        );
    }

    #[test]
    fn rejects_invalid_header() {
        let err = load_from_str("BAD\t1\n").expect_err("expected parse error");
        assert!(err.to_string().contains("expected header"));
    }

    #[test]
    fn rejects_duplicate_cell_records() {
        let input = "DVISICALC\t1\nCELL\tA1\tN\t1\nCELL\tA1\tN\t2\n";
        let err = load_from_str(input).expect_err("expected parse error");
        assert!(err.to_string().contains("duplicate cell"));
    }

    #[test]
    fn rejects_unknown_record_kind() {
        let input = "DVISICALC\t1\nWAT\tA1\tN\t1\n";
        let err = load_from_str(input).expect_err("expected parse error");
        assert!(err.to_string().contains("unknown record kind"));
    }

    #[test]
    fn supports_manual_mode_roundtrip() {
        let mut engine = Engine::new();
        engine.set_recalc_mode(RecalcMode::Manual);
        engine.set_number_a1("A1", 2.0).expect("set A1");

        let text = save_to_string(&engine).expect("serialize engine");
        let loaded = load_from_str(&text).expect("deserialize engine");
        assert_eq!(loaded.recalc_mode(), RecalcMode::Manual);
    }

    #[test]
    fn malformed_escape_fails() {
        let input = "DVISICALC\t1\nCELL\tA1\tF\t=A1+\\\n";
        let err = load_from_str(input).expect_err("expected parse error");
        assert!(err.to_string().contains("trailing escape"));
    }

    #[test]
    fn supports_utf8_bom_header() {
        let input = "\u{feff}DVISICALC\t1\nCELL\tA1\tN\t3\n";
        let loaded = load_from_str(input).expect("expected successful load");
        assert_eq!(
            loaded.cell_state_a1("A1").expect("query").value,
            Value::Number(3.0)
        );
    }

    #[test]
    fn reports_line_for_formula_load_error() {
        let input = "DVISICALC\t1\nCELL\tA1\tF\t=\n";
        let err = load_from_str(input).expect_err("expected formula parse error");
        let msg = err.to_string();
        assert!(msg.contains("line 2"));
        assert!(msg.contains("failed to apply cell"));
    }

    #[test]
    fn rejects_duplicate_name_records() {
        let input = "DVISICALC\t1\nNAME\tRATE\tN\t0.2\nNAME\tRATE\tN\t0.3\n";
        let err = load_from_str(input).expect_err("expected parse error");
        assert!(err.to_string().contains("duplicate name"));
    }

    #[test]
    fn reports_line_for_invalid_name_record() {
        let input = "DVISICALC\t1\nNAME\tA1\tN\t1\n";
        let err = load_from_str(input).expect_err("expected parse error");
        let msg = err.to_string();
        assert!(msg.contains("line 2"));
        assert!(msg.contains("failed to apply name"));
    }
}
