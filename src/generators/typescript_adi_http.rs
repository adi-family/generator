use super::{GeneratedOutput, Generator};
use crate::config::GenerationConfig;
use crate::parsers::{ParameterLocation, SchemaIR, TypeInfo};
use anyhow::Result;
use std::collections::HashMap;

pub struct TypeScriptAdiHttpGenerator;

impl Generator for TypeScriptAdiHttpGenerator {
    fn name(&self) -> &str {
        "typescript_adi_http"
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        let mut output = String::new();

        // Header
        output.push_str(&format!(
            "// Generated ADI HTTP Routes for {}\n",
            schema_ir.metadata.title
        ));
        output.push_str(&format!("// Version: {}\n\n", schema_ir.metadata.version));

        output.push_str("import { z } from 'zod';\n");
        output.push_str(
            "import { createRoute, createRouter, createClient } from '@adi-family/http';\n\n",
        );

        // Generate schemas
        output.push_str(
            "// ============================================================================\n",
        );
        output.push_str("// Schema Definitions\n");
        output.push_str(
            "// ============================================================================\n\n",
        );

        for schema in &schema_ir.schemas {
            if let Some(desc) = &schema.description {
                output.push_str(&format!("// {}\n", desc));
            }

            output.push_str(&format!(
                "export const {}Schema = z.object({{\n",
                schema.name
            ));

            for field in &schema.fields {
                let zod_type = type_info_to_zod(&field.type_info);
                let optional_suffix = if field.required { "" } else { ".optional()" };

                if let Some(desc) = &field.description {
                    output.push_str(&format!("  /** {} */\n", desc));
                }

                output.push_str(&format!(
                    "  {}: {}{},\n",
                    field.name, zod_type, optional_suffix
                ));
            }

            output.push_str("});\n\n");
            output.push_str(&format!(
                "export type {} = z.infer<typeof {}Schema>;\n\n",
                schema.name, schema.name
            ));
        }

        // Generate routes
        output.push_str(
            "// ============================================================================\n",
        );
        output.push_str("// Route Definitions\n");
        output.push_str(
            "// ============================================================================\n\n",
        );

        output.push_str("export const routes = {\n");

        for operation in &schema_ir.operations {
            if let Some(desc) = &operation.description {
                output.push_str(&format!("  // {}\n", desc));
            }

            output.push_str(&format!("  {}: createRoute({{\n", operation.id));
            output.push_str(&format!(
                "    method: '{}',\n",
                format!("{:?}", operation.method).to_uppercase()
            ));
            output.push_str(&format!("    path: '{}',\n", operation.path));

            // Query parameters
            let query_params: Vec<_> = operation
                .parameters
                .iter()
                .filter(|p| matches!(p.location, ParameterLocation::Query))
                .collect();

            if !query_params.is_empty() {
                output.push_str("    query: z.object({\n");
                for param in query_params {
                    let param_type = param_type_to_zod(&param.schema_type);
                    let optional = if param.required { "" } else { ".optional()" };
                    output.push_str(&format!(
                        "      {}: {}{},\n",
                        param.name, param_type, optional
                    ));
                }
                output.push_str("    }).optional(),\n");
            }

            // Path parameters
            let path_params: Vec<_> = operation
                .parameters
                .iter()
                .filter(|p| matches!(p.location, ParameterLocation::Path))
                .collect();

            if !path_params.is_empty() {
                output.push_str("    params: z.object({\n");
                for param in path_params {
                    let param_type = param_type_to_zod(&param.schema_type);
                    output.push_str(&format!("      {}: {},\n", param.name, param_type));
                }
                output.push_str("    }),\n");
            }

            // Request body (if POST/PUT/PATCH)
            if let Some(request_body) = &operation.request_body {
                output.push_str(&format!("    body: {}Schema,\n", request_body.name));
            }

            // Response
            if let Some(response) = &operation.response {
                output.push_str(&format!("    response: {}Schema,\n", response.name));
            } else {
                output.push_str("    response: z.void(),\n");
            }

            output.push_str("  }),\n\n");
        }

        output.push_str("};\n\n");

        // Generate server router (if enabled)
        let include_server = config
            .options
            .get("includeServer")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if include_server {
            let router_name = config
                .options
                .get("routerName")
                .and_then(|v| v.as_str())
                .unwrap_or("apiRouter");

            output.push_str(
                "// ============================================================================\n",
            );
            output.push_str("// Server-side Router\n");
            output.push_str("// ============================================================================\n\n");

            output.push_str(&format!(
                "export const {} = createRouter(routes, {{\n",
                router_name
            ));

            for operation in &schema_ir.operations {
                output.push_str(&format!("  {}: async (req) => {{\n", operation.id));
                output.push_str("    // TODO: Implement handler\n");

                // Add type hints in comments
                if !operation.parameters.is_empty() {
                    output.push_str("    // Request parameters:\n");
                    for param in &operation.parameters {
                        output.push_str(&format!(
                            "    //   req.{}: {}\n",
                            format!("{:?}", param.location).to_lowercase(),
                            param.schema_type
                        ));
                    }
                }

                if let Some(response) = &operation.response {
                    output.push_str(&format!("    // Must return: {}\n", response.name));
                }

                output.push_str("    throw new Error('Not implemented');\n");
                output.push_str("  },\n\n");
            }

            output.push_str("});\n\n");
        }

        // Generate client (if enabled)
        let include_client = config
            .options
            .get("includeClient")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);

        if include_client {
            let client_name = config
                .options
                .get("clientName")
                .and_then(|v| v.as_str())
                .unwrap_or("apiClient");

            let base_url_env = config
                .options
                .get("baseUrlEnvVar")
                .and_then(|v| v.as_str())
                .unwrap_or("API_BASE_URL");

            output.push_str(
                "// ============================================================================\n",
            );
            output.push_str("// Client-side API\n");
            output.push_str("// ============================================================================\n\n");

            output.push_str(&format!(
                "export const {} = createClient(routes, {{\n",
                client_name
            ));
            output.push_str(&format!(
                "  baseUrl: process.env.{} || '{}',\n",
                base_url_env,
                schema_ir
                    .metadata
                    .base_url
                    .as_deref()
                    .unwrap_or("http://localhost:3000")
            ));
            output.push_str("});\n\n");

            // Add usage examples
            output.push_str("// Usage examples:\n");
            for operation in schema_ir.operations.iter().take(2) {
                output.push_str(&format!(
                    "// const result = await {}.{}(",
                    client_name, operation.id
                ));

                let has_params = !operation.parameters.is_empty();
                let has_body = operation.request_body.is_some();

                if has_params || has_body {
                    output.push_str("{ ");

                    if has_params {
                        for param in operation.parameters.iter().take(1) {
                            output.push_str(&format!("{}: value", param.name));
                        }
                    }

                    output.push_str(" }");
                }

                output.push_str(");\n");
            }
        }

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content: output,
            metadata: HashMap::new(),
        })
    }
}

// Note: This function intentionally differs from TypeInfo::to_typescript_zod() for ADI HTTP-specific needs:
// 1. Adds "Schema" suffix to reference names (e.g., "UserSchema" instead of "User")
// 2. Uses z.string().datetime() for dates (not z.date().or(z.string()))
// 3. Uses z.number().int() for integers (not just z.number())
// 4. Uses z.record(z.any()) for objects (not z.any())
// These differences are required for @adi-family/http compatibility.
fn type_info_to_zod(type_info: &TypeInfo) -> String {
    if type_info.is_array {
        if let Some(item_type) = &type_info.array_item_type {
            return format!("z.array({})", type_info_to_zod(item_type));
        }
        return "z.array(z.any())".to_string();
    }

    // ADI HTTP-specific: Add "Schema" suffix to references
    if let Some(ref_name) = &type_info.reference {
        return format!("{}Schema", ref_name);
    }

    if let Some(enum_vals) = &type_info.enum_values {
        let values: Vec<String> = enum_vals.iter().map(|v| format!("\"{}\"", v)).collect();
        return format!("z.enum([{}])", values.join(", "));
    }

    match type_info.openapi_type.as_str() {
        "string" => {
            if let Some(fmt) = &type_info.format {
                match fmt.as_str() {
                    "email" => "z.string().email()".to_string(),
                    "uuid" => "z.string().uuid()".to_string(),
                    "uri" | "url" => "z.string().url()".to_string(),
                    "date" | "date-time" => "z.string().datetime()".to_string(),
                    _ => "z.string()".to_string(),
                }
            } else {
                "z.string()".to_string()
            }
        }
        "integer" => "z.number().int()".to_string(),
        "number" => "z.number()".to_string(),
        "boolean" => "z.boolean()".to_string(),
        "object" => "z.record(z.any())".to_string(),
        _ => "z.any()".to_string(),
    }
}

fn param_type_to_zod(schema_type: &str) -> String {
    match schema_type {
        "integer" => "z.coerce.number().int()".to_string(),
        "number" => "z.coerce.number()".to_string(),
        "boolean" => "z.coerce.boolean()".to_string(),
        _ => "z.string()".to_string(),
    }
}
