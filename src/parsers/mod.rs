pub mod schema_ir;
pub mod openapi_parser;

use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;

pub use schema_ir::*;
pub use openapi_parser::OpenApiParser;

/// Input parser trait - converts any format to unified IR
pub trait InputParser: Send + Sync {
    /// Name of the input format (e.g., "openapi", "typescript", "graphql")
    fn format_name(&self) -> &str;

    /// File extensions this parser supports
    fn supported_extensions(&self) -> Vec<&str>;

    /// Parse input file into intermediate representation
    fn parse(&self, source: &Path, options: &HashMap<String, Value>) -> Result<SchemaIR>;

    /// Validate input file before parsing
    fn validate(&self, source: &Path) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Input file not found: {:?}", source);
        }
        Ok(())
    }
}

/// Parser registry for managing available input parsers
pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn InputParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };

        // Register built-in parsers
        registry.register(Box::new(OpenApiParser));

        registry
    }

    pub fn register(&mut self, parser: Box<dyn InputParser>) {
        self.parsers.insert(parser.format_name().to_string(), parser);
    }

    pub fn get(&self, format: &str) -> Option<&Box<dyn InputParser>> {
        self.parsers.get(format)
    }

    /// Auto-detect format from file extension
    pub fn detect_format(&self, path: &Path) -> Option<&str> {
        let ext = path.extension()?.to_str()?;

        for parser in self.parsers.values() {
            if parser.supported_extensions().contains(&ext) {
                return Some(parser.format_name());
            }
        }

        None
    }
}

impl Default for ParserRegistry {
    fn default() -> Self {
        Self::new()
    }
}
