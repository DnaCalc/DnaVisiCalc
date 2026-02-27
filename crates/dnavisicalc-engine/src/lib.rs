pub mod config {
    use std::fmt;

    pub const COREENGINE_ENV: &str = "DNAVISICALC_COREENGINE";

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum CoreEngineId {
        RustCore,
    }

    impl CoreEngineId {
        pub const fn as_str(self) -> &'static str {
            match self {
                Self::RustCore => "rust-core",
            }
        }

        pub fn parse(input: &str) -> Option<Self> {
            let normalized = input.trim().to_ascii_lowercase();
            match normalized.as_str() {
                "rust" | "rust-core" | "core" => Some(Self::RustCore),
                _ => None,
            }
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EngineConfig {
        pub coreengine: CoreEngineId,
    }

    impl Default for EngineConfig {
        fn default() -> Self {
            Self {
                coreengine: CoreEngineId::RustCore,
            }
        }
    }

    impl EngineConfig {
        pub fn from_env_lossy() -> Self {
            let Some(raw) = std::env::var_os(COREENGINE_ENV) else {
                return Self::default();
            };
            CoreEngineId::parse(&raw.to_string_lossy())
                .map(|coreengine| Self { coreengine })
                .unwrap_or_default()
        }

        pub fn from_env_strict() -> Result<Self, EngineConfigError> {
            let Some(raw) = std::env::var_os(COREENGINE_ENV) else {
                return Ok(Self::default());
            };
            let raw_text = raw.to_string_lossy().to_string();
            let Some(coreengine) = CoreEngineId::parse(&raw_text) else {
                return Err(EngineConfigError::UnknownCoreEngine(raw_text));
            };
            Ok(Self { coreengine })
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum EngineConfigError {
        UnknownCoreEngine(String),
    }

    impl fmt::Display for EngineConfigError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::UnknownCoreEngine(value) => write!(
                    f,
                    "unknown coreengine '{value}' (supported: rust-core; aliases: rust, core)"
                ),
            }
        }
    }

    impl std::error::Error for EngineConfigError {}

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parse_aliases_for_rust_core() {
            assert_eq!(CoreEngineId::parse("rust"), Some(CoreEngineId::RustCore));
            assert_eq!(
                CoreEngineId::parse("rust-core"),
                Some(CoreEngineId::RustCore)
            );
            assert_eq!(CoreEngineId::parse("core"), Some(CoreEngineId::RustCore));
        }

        #[test]
        fn parse_unknown_coreengine_returns_none() {
            assert_eq!(CoreEngineId::parse("unknown"), None);
        }
    }
}

use std::ops::{Deref, DerefMut};

pub use config::{COREENGINE_ENV, CoreEngineId, EngineConfig, EngineConfigError};
pub use dnavisicalc_core::{
    AddressError, BinaryOp, CalcNode, CalcTree, CellError, CellFormat, CellInput, CellRange,
    CellRef, CellState, ChangeEntry, ChartDefinition, ChartOutput, ChartSeriesOutput,
    ControlDefinition, ControlKind, DEFAULT_SHEET_BOUNDS, DependencyError, DynamicArrayStrategy,
    EngineError, Expr, FnUdf, FnUdfWithVolatility, IterationConfig, MAX_COLUMNS, MAX_ROWS,
    NameInput, PaletteColor, ParseError, RecalcMode, RefFlags, SUPPORTED_FUNCTIONS, Scc,
    SheetBounds, StructuralOp, UdfHandler, UnaryOp, Value, Volatility, build_calc_tree,
    build_calc_tree_allow_cycles, col_index_to_label, col_label_to_index, expr_to_formula,
    parse_formula, rewrite_expr,
};

#[derive(Debug)]
pub struct Engine {
    config: EngineConfig,
    inner: dnavisicalc_core::Engine,
}

impl Default for Engine {
    fn default() -> Self {
        Self::new()
    }
}

impl Engine {
    pub fn new() -> Self {
        let config = EngineConfig::from_env_lossy();
        Self::new_with_config(config)
    }

    pub fn with_bounds(bounds: SheetBounds) -> Self {
        let config = EngineConfig::from_env_lossy();
        Self::with_bounds_and_config(bounds, config)
    }

    pub fn new_with_config(config: EngineConfig) -> Self {
        let inner = match config.coreengine {
            CoreEngineId::RustCore => dnavisicalc_core::Engine::new(),
        };
        Self { config, inner }
    }

    pub fn with_bounds_and_config(bounds: SheetBounds, config: EngineConfig) -> Self {
        let inner = match config.coreengine {
            CoreEngineId::RustCore => dnavisicalc_core::Engine::with_bounds(bounds),
        };
        Self { config, inner }
    }

    pub fn coreengine(&self) -> CoreEngineId {
        self.config.coreengine
    }

    pub fn engine_config(&self) -> EngineConfig {
        self.config
    }

    pub fn into_inner(self) -> dnavisicalc_core::Engine {
        self.inner
    }

    pub fn inner(&self) -> &dnavisicalc_core::Engine {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut dnavisicalc_core::Engine {
        &mut self.inner
    }
}

impl From<dnavisicalc_core::Engine> for Engine {
    fn from(inner: dnavisicalc_core::Engine) -> Self {
        Self {
            config: EngineConfig::default(),
            inner,
        }
    }
}

impl From<Engine> for dnavisicalc_core::Engine {
    fn from(value: Engine) -> Self {
        value.inner
    }
}

impl Deref for Engine {
    type Target = dnavisicalc_core::Engine;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Engine {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
