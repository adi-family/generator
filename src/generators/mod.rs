pub mod typescript;
pub mod typescript_adi_http;
pub mod python;
pub mod golang;

use anyhow::Result;
use crate::config::GenerationConfig;
use crate::parsers::SchemaIR;
use std::collections::HashMap;

pub use typescript::TypeScriptGenerator;
pub use typescript_adi_http::TypeScriptAdiHttpGenerator;
pub use python::PythonGenerator;
pub use golang::GolangGenerator;

/// Generated output from a generator
#[derive(Debug)]
pub struct GeneratedOutput {
    pub filename: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}

/// Generator trait - converts SchemaIR to target language code
pub trait Generator: Send + Sync {
    /// Unique name of the generator (e.g., "typescript", "python")
    fn name(&self) -> &str;

    /// File extension for generated output (e.g., "ts", "py")
    fn file_extension(&self) -> &str;

    /// Generate code from intermediate representation
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput>;

    /// Validate generator-specific configuration
    fn validate_config(&self, _config: &GenerationConfig) -> Result<()> {
        Ok(())
    }
}

/// Generator registry for managing available code generators
pub struct GeneratorRegistry {
    generators: HashMap<String, Box<dyn Generator>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            generators: HashMap::new(),
        };

        // Register built-in generators
        registry.register(Box::new(TypeScriptGenerator));
        registry.register(Box::new(TypeScriptAdiHttpGenerator));
        registry.register(Box::new(PythonGenerator));
        registry.register(Box::new(GolangGenerator));

        registry
    }

    pub fn register(&mut self, generator: Box<dyn Generator>) {
        self.generators.insert(generator.name().to_string(), generator);
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Generator>> {
        self.generators.get(name)
    }

    pub fn available_generators(&self) -> Vec<&str> {
        self.generators.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for GeneratorRegistry {
    fn default() -> Self {
        Self::new()
    }
}
