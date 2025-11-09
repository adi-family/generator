use super::{Generator, GeneratedOutput};
use crate::config::GenerationConfig;
use crate::parsers::{SchemaIR, TypeInfo};
use anyhow::Result;
use std::collections::HashMap;
use tera::{Tera, Context};

pub struct TypeScriptGenerator;

impl Generator for TypeScriptGenerator {
    fn name(&self) -> &str {
        "typescript"
    }

    fn file_extension(&self) -> &str {
        "ts"
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
            .unwrap_or("templates/typescript");

        let tera = Tera::new(&format!("{}/**/*.tera", template_path))?;
        let mut context = Context::new();

        // Add metadata
        context.insert("api_title", &schema_ir.metadata.title);
        context.insert("api_version", &schema_ir.metadata.version);
        context.insert("base_url", &schema_ir.metadata.base_url.clone().unwrap_or_else(|| "http://localhost".to_string()));

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
                            "typescript_type": type_info_to_typescript_zod(&field.type_info),
                            "required": field.required,
                            "nullable": false,
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

        // Convert operations for template
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

        // Add generator options
        context.insert("options", &config.options);

        // Render template
        let content = tera.render("client.ts.tera", &context)?;

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content,
            metadata: HashMap::new(),
        })
    }
}

fn type_info_to_typescript_zod(type_info: &TypeInfo) -> String {
    if type_info.is_array {
        if let Some(item_type) = &type_info.array_item_type {
            return format!("z.array({})", type_info_to_typescript_zod(item_type));
        }
        return "z.array(z.any())".to_string();
    }

    if let Some(ref_name) = &type_info.reference {
        return ref_name.clone();
    }

    if let Some(enum_vals) = &type_info.enum_values {
        let values: Vec<String> = enum_vals.iter().map(|v| format!("\"{}\"", v)).collect();
        return format!("z.enum([{}])", values.join(", "));
    }

    match type_info.openapi_type.as_str() {
        "string" => {
            if let Some(fmt) = &type_info.format {
                match fmt.as_str() {
                    "date" | "date-time" => "z.date().or(z.string())".to_string(),
                    "email" => "z.string().email()".to_string(),
                    "uuid" => "z.string().uuid()".to_string(),
                    "uri" => "z.string().url()".to_string(),
                    _ => "z.string()".to_string(),
                }
            } else {
                "z.string()".to_string()
            }
        }
        "integer" | "number" => "z.number()".to_string(),
        "boolean" => "z.boolean()".to_string(),
        "object" => "z.any()".to_string(),
        _ => "z.any()".to_string(),
    }
}
