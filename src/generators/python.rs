use super::{GeneratedOutput, Generator};
use crate::config::GenerationConfig;
use crate::parsers::SchemaIR;
use anyhow::Result;
use std::collections::HashMap;
use tera::{Context, Tera};

pub struct PythonGenerator;

impl Generator for PythonGenerator {
    fn name(&self) -> &str {
        "python"
    }

    fn file_extension(&self) -> &str {
        "py"
    }

    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        // Determine template path
        let template_path = config
            .template
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("templates/python");

        let tera = Tera::new(&format!("{}/**/*.tera", template_path))?;
        let mut context = Context::new();

        // Add metadata
        context.insert("api_title", &schema_ir.metadata.title);
        context.insert("api_version", &schema_ir.metadata.version);
        context.insert(
            "base_url",
            &schema_ir
                .metadata
                .base_url
                .clone()
                .unwrap_or_else(|| "http://localhost".to_string()),
        );

        // Convert schemas for template
        let schemas_for_template: Vec<_> = schema_ir
            .schemas
            .iter()
            .map(|schema| {
                let properties: Vec<_> = schema
                    .fields
                    .iter()
                    .map(|field| {
                        serde_json::json!({
                            "name": field.name,
                            "python_type": field.type_info.to_python(),
                            "required": field.required,
                        })
                    })
                    .collect();

                serde_json::json!({
                    "name": schema.name,
                    "properties": properties,
                    "description": schema.description,
                })
            })
            .collect();

        context.insert("schemas", &schemas_for_template);

        // Convert operations
        let operations_for_template: Vec<_> = schema_ir
            .operations
            .iter()
            .map(|op| {
                serde_json::json!({
                    "id": op.id,
                    "method": format!("{:?}", op.method).to_uppercase(),
                    "path": op.path,
                    "parameters": op.parameters.iter().map(|p| {
                        serde_json::json!({
                            "name": p.name,
                            "location": format!("{:?}", p.location).to_lowercase(),
                            "required": p.required,
                            "schema_type": p.schema_type,
                        })
                    }).collect::<Vec<_>>(),
                    "responses": serde_json::json!([]),  // TODO: populate from op.response
                })
            })
            .collect();

        context.insert("operations", &operations_for_template);
        context.insert("options", &config.options);

        // Render template
        let content = tera.render("client.py.tera", &context)?;

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content,
            metadata: HashMap::new(),
        })
    }
}
