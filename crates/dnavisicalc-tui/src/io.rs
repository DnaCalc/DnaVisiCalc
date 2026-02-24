use std::collections::HashMap;

use dnavisicalc_core::Engine;

pub trait WorkbookIo {
    fn load(&mut self, path: &str) -> Result<Engine, String>;
    fn save(&mut self, path: &str, engine: &Engine) -> Result<(), String>;
}

#[derive(Debug, Default)]
pub struct FsWorkbookIo;

impl WorkbookIo for FsWorkbookIo {
    fn load(&mut self, path: &str) -> Result<Engine, String> {
        dnavisicalc_file::load_from_path(path).map_err(|err| err.to_string())
    }

    fn save(&mut self, path: &str, engine: &Engine) -> Result<(), String> {
        dnavisicalc_file::save_to_path(engine, path).map_err(|err| err.to_string())
    }
}

#[derive(Debug, Default)]
pub struct MemoryWorkbookIo {
    files: HashMap<String, String>,
}

impl MemoryWorkbookIo {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn files(&self) -> &HashMap<String, String> {
        &self.files
    }
}

impl WorkbookIo for MemoryWorkbookIo {
    fn load(&mut self, path: &str) -> Result<Engine, String> {
        let content = self
            .files
            .get(path)
            .ok_or_else(|| format!("file not found: {path}"))?
            .clone();
        dnavisicalc_file::load_from_str(&content).map_err(|err| err.to_string())
    }

    fn save(&mut self, path: &str, engine: &Engine) -> Result<(), String> {
        let content = dnavisicalc_file::save_to_string(engine).map_err(|err| err.to_string())?;
        self.files.insert(path.to_string(), content);
        Ok(())
    }
}
