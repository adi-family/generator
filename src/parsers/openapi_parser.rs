use super::{InputParser, SchemaIR, OriginalData, Metadata, SchemaDefinition, FieldDefinition, TypeInfo};
use super::{OperationDefinition, HttpMethod, Parameter, ParameterLocation};
use anyhow::{Context, Result};
use openapiv3::{OpenAPI, ReferenceOr, Schema, SchemaKind, Type, Operation, PathItem};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub struct OpenApiParser;

impl InputParser for OpenApiParser {
    fn format_name(&self) -> &str {
        "openapi"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml", "json"]
    }

    fn parse(&self, source: &Path, _options: &HashMap<String, Value>) -> Result<SchemaIR> {
        self.validate(source)?;

        let content = fs::read_to_string(source)
            .with_context(|| format!("Failed to read OpenAPI spec: {:?}", source))?;

        let openapi: OpenAPI = if source.extension().and_then(|s| s.to_str()) == Some("json") {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        // Serialize full OpenAPI spec to JSON for original data
        let original_json = serde_json::to_value(&openapi)?;

        // Extract custom metadata (x- extensions)
        let mut custom_metadata = HashMap::new();
        for (key, value) in &openapi.info.extensions {
            custom_metadata.insert(key.clone(), value.clone());
        }

        // Build SchemaIR
        Ok(SchemaIR {
            metadata: Metadata {
                title: openapi.info.title.clone(),
                version: openapi.info.version.clone(),
                description: openapi.info.description.clone(),
                base_url: openapi
                    .servers
                    .first()
                    .map(|s| s.url.clone()),
                custom: custom_metadata,
            },
            schemas: extract_schemas(&openapi)?,
            operations: extract_operations(&openapi)?,
            original: OriginalData {
                format: "openapi".to_string(),
                data: original_json,
                extensions: extract_global_extensions(&openapi),
            },
        })
    }
}

fn extract_schemas(openapi: &OpenAPI) -> Result<Vec<SchemaDefinition>> {
    let mut schemas = Vec::new();

    if let Some(components) = &openapi.components {
        for (schema_name, schema_ref) in &components.schemas {
            if let ReferenceOr::Item(schema) = schema_ref {
                let original_json = serde_json::to_value(schema)?;

                schemas.push(SchemaDefinition {
                    name: schema_name.clone(),
                    fields: extract_fields(schema)?,
                    description: schema.schema_data.description.clone(),
                    original: original_json,
                });
            }
        }
    }

    Ok(schemas)
}

fn extract_fields(schema: &Schema) -> Result<Vec<FieldDefinition>> {
    let mut fields = Vec::new();

    if let SchemaKind::Type(Type::Object(obj_type)) = &schema.schema_kind {
        let required = &obj_type.required;

        for (field_name, field_schema_ref) in &obj_type.properties {
            let is_required = required.contains(field_name);

            let field_schema = match field_schema_ref {
                ReferenceOr::Item(schema_box) => schema_box.as_ref(),
                ReferenceOr::Reference { reference } => {
                    // Handle references
                    let ref_name = reference.split('/').last().unwrap_or("Unknown");
                    let original_json = serde_json::json!({ "$ref": reference });

                    fields.push(FieldDefinition {
                        name: field_name.clone(),
                        type_info: TypeInfo {
                            openapi_type: "object".to_string(),
                            format: None,
                            is_array: false,
                            array_item_type: None,
                            reference: Some(ref_name.to_string()),
                            enum_values: None,
                        },
                        required: is_required,
                        description: None,
                        original: original_json,
                    });
                    continue;
                }
            };

            let original_json = serde_json::to_value(field_schema)?;
            let type_info = extract_type_info(field_schema);

            fields.push(FieldDefinition {
                name: field_name.clone(),
                type_info,
                required: is_required,
                description: field_schema.schema_data.description.clone(),
                original: original_json,
            });
        }
    }

    Ok(fields)
}

fn extract_type_info(schema: &Schema) -> TypeInfo {
    match &schema.schema_kind {
        SchemaKind::Type(Type::String(string_type)) => {
            let format_str = match &string_type.format {
                openapiv3::VariantOrUnknownOrEmpty::Item(fmt) => Some(format!("{:?}", fmt)),
                _ => None,
            };

            TypeInfo {
                openapi_type: "string".to_string(),
                format: format_str,
                is_array: false,
                array_item_type: None,
                reference: None,
                enum_values: if !string_type.enumeration.is_empty() {
                    Some(
                        string_type
                            .enumeration
                            .iter()
                            .filter_map(|v| v.clone())
                            .collect(),
                    )
                } else {
                    None
                },
            }
        }
        SchemaKind::Type(Type::Number(num_type)) => {
            let format_str = match &num_type.format {
                openapiv3::VariantOrUnknownOrEmpty::Item(fmt) => Some(format!("{:?}", fmt)),
                _ => None,
            };

            TypeInfo {
                openapi_type: "number".to_string(),
                format: format_str,
                is_array: false,
                array_item_type: None,
                reference: None,
                enum_values: None,
            }
        }
        SchemaKind::Type(Type::Integer(int_type)) => {
            let format_str = match &int_type.format {
                openapiv3::VariantOrUnknownOrEmpty::Item(fmt) => Some(format!("{:?}", fmt)),
                _ => None,
            };

            TypeInfo {
                openapi_type: "integer".to_string(),
                format: format_str,
                is_array: false,
                array_item_type: None,
                reference: None,
                enum_values: None,
            }
        }
        SchemaKind::Type(Type::Boolean(_)) => TypeInfo {
            openapi_type: "boolean".to_string(),
            format: None,
            is_array: false,
            array_item_type: None,
            reference: None,
            enum_values: None,
        },
        SchemaKind::Type(Type::Array(array_type)) => {
            let item_type = if let Some(items) = &array_type.items {
                match items {
                    ReferenceOr::Item(item_schema) => {
                        Box::new(extract_type_info(item_schema))
                    }
                    ReferenceOr::Reference { reference } => {
                        let ref_name = reference.split('/').last().unwrap_or("Unknown");
                        Box::new(TypeInfo {
                            openapi_type: "object".to_string(),
                            format: None,
                            is_array: false,
                            array_item_type: None,
                            reference: Some(ref_name.to_string()),
                            enum_values: None,
                        })
                    }
                }
            } else {
                Box::new(TypeInfo {
                    openapi_type: "any".to_string(),
                    format: None,
                    is_array: false,
                    array_item_type: None,
                    reference: None,
                    enum_values: None,
                })
            };

            TypeInfo {
                openapi_type: "array".to_string(),
                format: None,
                is_array: true,
                array_item_type: Some(item_type),
                reference: None,
                enum_values: None,
            }
        }
        SchemaKind::Type(Type::Object(_)) => TypeInfo {
            openapi_type: "object".to_string(),
            format: None,
            is_array: false,
            array_item_type: None,
            reference: None,
            enum_values: None,
        },
        _ => TypeInfo {
            openapi_type: "any".to_string(),
            format: None,
            is_array: false,
            array_item_type: None,
            reference: None,
            enum_values: None,
        },
    }
}

fn extract_operations(openapi: &OpenAPI) -> Result<Vec<OperationDefinition>> {
    let mut operations = Vec::new();

    for (path, path_item_ref) in &openapi.paths.paths {
        if let ReferenceOr::Item(path_item) = path_item_ref {
            extract_operations_from_path(path, path_item, &mut operations)?;
        }
    }

    Ok(operations)
}

fn extract_operations_from_path(
    path: &str,
    path_item: &PathItem,
    operations: &mut Vec<OperationDefinition>,
) -> Result<()> {
    let ops = vec![
        (&path_item.get, HttpMethod::Get),
        (&path_item.post, HttpMethod::Post),
        (&path_item.put, HttpMethod::Put),
        (&path_item.delete, HttpMethod::Delete),
        (&path_item.patch, HttpMethod::Patch),
        (&path_item.head, HttpMethod::Head),
        (&path_item.options, HttpMethod::Options),
    ];

    for (op_option, method) in ops {
        if let Some(operation) = op_option {
            let op_def = extract_operation(path, method, operation)?;
            operations.push(op_def);
        }
    }

    Ok(())
}

fn extract_operation(
    path: &str,
    method: HttpMethod,
    operation: &Operation,
) -> Result<OperationDefinition> {
    let original_json = serde_json::to_value(operation)?;

    let parameters = operation
        .parameters
        .iter()
        .filter_map(|param_ref| {
            if let ReferenceOr::Item(param) = param_ref {
                Some(Parameter {
                    name: param.parameter_data_ref().name.clone(),
                    location: match param {
                        openapiv3::Parameter::Query { .. } => ParameterLocation::Query,
                        openapiv3::Parameter::Header { .. } => ParameterLocation::Header,
                        openapiv3::Parameter::Path { .. } => ParameterLocation::Path,
                        openapiv3::Parameter::Cookie { .. } => ParameterLocation::Cookie,
                    },
                    required: param.parameter_data_ref().required,
                    schema_type: "string".to_string(), // Simplified for now
                    description: param.parameter_data_ref().description.clone(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(OperationDefinition {
        id: operation
            .operation_id
            .clone()
            .unwrap_or_else(|| format!("{}_{}", format!("{:?}", method).to_lowercase(), path.replace('/', "_"))),
        method,
        path: path.to_string(),
        parameters,
        request_body: None, // TODO: extract request body
        response: None,     // TODO: extract response
        description: operation.description.clone(),
        tags: operation.tags.clone(),
        original: original_json,
    })
}

fn extract_global_extensions(openapi: &OpenAPI) -> HashMap<String, Value> {
    let mut extensions = HashMap::new();

    for (key, value) in &openapi.extensions {
        extensions.insert(format!("openapi.{}", key), value.clone());
    }

    extensions
}
