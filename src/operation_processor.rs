use openapiv3::{OpenAPI, Operation, Parameter, ParameterSchemaOrContent, ReferenceOr, RequestBody, Response, MediaType};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedOperation {
    pub id: String,
    pub method: String,
    pub path: String,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub parameters: Vec<ProcessedParameter>,
    pub request_body: Option<ProcessedRequestBody>,
    pub responses: Vec<ProcessedResponse>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedParameter {
    pub name: String,
    pub location: String, // query, path, header, cookie
    pub required: bool,
    pub schema_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedRequestBody {
    pub required: bool,
    pub content_type: String,
    pub schema_ref: Option<String>,
    pub schema_type: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedResponse {
    pub status_code: String,
    pub description: String,
    pub content_type: Option<String>,
    pub schema_ref: Option<String>,
    pub schema_type: Option<String>,
    pub is_array: bool,
}

pub fn extract_operations(openapi: &OpenAPI) -> Vec<ProcessedOperation> {
    let mut operations = Vec::new();

    for (path, path_item) in &openapi.paths.paths {
        if let ReferenceOr::Item(item) = path_item {
            let ops = vec![
                ("get", &item.get),
                ("post", &item.post),
                ("put", &item.put),
                ("delete", &item.delete),
                ("patch", &item.patch),
            ];

            for (method, op_option) in ops {
                if let Some(operation) = op_option {
                    operations.push(process_operation(
                        operation,
                        method.to_string(),
                        path.clone(),
                    ));
                }
            }
        }
    }

    operations
}

fn process_operation(operation: &Operation, method: String, path: String) -> ProcessedOperation {
    let id = operation
        .operation_id
        .clone()
        .unwrap_or_else(|| format!("{}_{}", method, path.replace('/', "_").replace(['{', '}'], "")));

    let parameters = operation
        .parameters
        .iter()
        .filter_map(process_parameter)
        .collect();

    let request_body = operation.request_body.as_ref().and_then(process_request_body);

    let responses = operation
        .responses
        .responses
        .iter()
        .filter_map(|(status, resp)| process_response(status.to_string(), resp))
        .collect();

    ProcessedOperation {
        id,
        method,
        path,
        summary: operation.summary.clone(),
        description: operation.description.clone(),
        parameters,
        request_body,
        responses,
    }
}

fn process_parameter(param_ref: &ReferenceOr<Parameter>) -> Option<ProcessedParameter> {
    match param_ref {
        ReferenceOr::Item(param) => {
            let (name, location, required, schema_type, description) = match param {
                Parameter::Query { parameter_data, .. } => {
                    let schema_type = extract_parameter_type(&parameter_data.format);
                    (
                        parameter_data.name.clone(),
                        "query".to_string(),
                        parameter_data.required,
                        schema_type,
                        parameter_data.description.clone(),
                    )
                }
                Parameter::Path { parameter_data, .. } => {
                    let schema_type = extract_parameter_type(&parameter_data.format);
                    (
                        parameter_data.name.clone(),
                        "path".to_string(),
                        parameter_data.required,
                        schema_type,
                        parameter_data.description.clone(),
                    )
                }
                Parameter::Header { parameter_data, .. } => {
                    let schema_type = extract_parameter_type(&parameter_data.format);
                    (
                        parameter_data.name.clone(),
                        "header".to_string(),
                        parameter_data.required,
                        schema_type,
                        parameter_data.description.clone(),
                    )
                }
                Parameter::Cookie { parameter_data, .. } => {
                    let schema_type = extract_parameter_type(&parameter_data.format);
                    (
                        parameter_data.name.clone(),
                        "cookie".to_string(),
                        parameter_data.required,
                        schema_type,
                        parameter_data.description.clone(),
                    )
                }
            };

            Some(ProcessedParameter {
                name,
                location,
                required,
                schema_type,
                description,
            })
        }
        ReferenceOr::Reference { .. } => None,
    }
}

fn extract_parameter_type(format: &ParameterSchemaOrContent) -> String {
    match format {
        ParameterSchemaOrContent::Schema(schema_ref) => match schema_ref {
            ReferenceOr::Item(schema) => {
                use openapiv3::{SchemaKind, Type};
                match &schema.schema_kind {
                    SchemaKind::Type(Type::String(_)) => "string".to_string(),
                    SchemaKind::Type(Type::Number(_)) => "number".to_string(),
                    SchemaKind::Type(Type::Integer(_)) => "integer".to_string(),
                    SchemaKind::Type(Type::Boolean(_)) => "boolean".to_string(),
                    SchemaKind::Type(Type::Array(_)) => "array".to_string(),
                    _ => "any".to_string(),
                }
            }
            ReferenceOr::Reference { .. } => "any".to_string(),
        },
        ParameterSchemaOrContent::Content(_) => "any".to_string(),
    }
}

fn process_request_body(body_ref: &ReferenceOr<RequestBody>) -> Option<ProcessedRequestBody> {
    match body_ref {
        ReferenceOr::Item(body) => {
            let content_type = body
                .content
                .keys()
                .next()
                .map(|s| s.to_string())
                .unwrap_or_else(|| "application/json".to_string());

            let (schema_ref, schema_type) = body
                .content
                .get(&content_type)
                .and_then(extract_schema_info)
                .unwrap_or((None, None));

            Some(ProcessedRequestBody {
                required: body.required,
                content_type,
                schema_ref,
                schema_type,
            })
        }
        ReferenceOr::Reference { .. } => None,
    }
}

fn process_response(
    status: String,
    response_ref: &ReferenceOr<Response>,
) -> Option<ProcessedResponse> {
    match response_ref {
        ReferenceOr::Item(response) => {
            let description = response.description.clone();

            let content_type = response.content.keys().next().map(|s| s.to_string());

            let (schema_ref, schema_type, is_array) = content_type
                .as_ref()
                .and_then(|ct| response.content.get(ct))
                .and_then(extract_schema_info_with_array)
                .unwrap_or((None, None, false));

            Some(ProcessedResponse {
                status_code: status,
                description,
                content_type,
                schema_ref,
                schema_type,
                is_array,
            })
        }
        ReferenceOr::Reference { .. } => None,
    }
}

fn extract_schema_info(media_type: &MediaType) -> Option<(Option<String>, Option<String>)> {
    media_type.schema.as_ref().map(|schema_ref| {
        match schema_ref {
            ReferenceOr::Reference { reference } => {
                let schema_name = reference.split('/').next_back().unwrap_or("Unknown");
                (Some(schema_name.to_string()), Some("object".to_string()))
            }
            ReferenceOr::Item(schema) => {
                use openapiv3::{SchemaKind, Type};
                let type_str = match &schema.schema_kind {
                    SchemaKind::Type(Type::String(_)) => "string",
                    SchemaKind::Type(Type::Number(_)) => "number",
                    SchemaKind::Type(Type::Integer(_)) => "integer",
                    SchemaKind::Type(Type::Boolean(_)) => "boolean",
                    SchemaKind::Type(Type::Array(_)) => "array",
                    SchemaKind::Type(Type::Object(_)) => "object",
                    _ => "any",
                };
                (None, Some(type_str.to_string()))
            }
        }
    })
}

fn extract_schema_info_with_array(media_type: &MediaType) -> Option<(Option<String>, Option<String>, bool)> {
    media_type.schema.as_ref().map(|schema_ref| {
        match schema_ref {
            ReferenceOr::Reference { reference } => {
                let schema_name = reference.split('/').next_back().unwrap_or("Unknown");
                (Some(schema_name.to_string()), Some("object".to_string()), false)
            }
            ReferenceOr::Item(schema) => {
                use openapiv3::{SchemaKind, Type};
                match &schema.schema_kind {
                    SchemaKind::Type(Type::Array(array_type)) => {
                        if let Some(items) = &array_type.items {
                            match items {
                                ReferenceOr::Reference { reference } => {
                                    let schema_name = reference.split('/').next_back().unwrap_or("Unknown");
                                    (Some(schema_name.to_string()), Some("object".to_string()), true)
                                }
                                ReferenceOr::Item(_) => {
                                    (None, Some("any".to_string()), true)
                                }
                            }
                        } else {
                            (None, Some("any".to_string()), true)
                        }
                    }
                    SchemaKind::Type(Type::String(_)) => (None, Some("string".to_string()), false),
                    SchemaKind::Type(Type::Number(_)) => (None, Some("number".to_string()), false),
                    SchemaKind::Type(Type::Integer(_)) => (None, Some("integer".to_string()), false),
                    SchemaKind::Type(Type::Boolean(_)) => (None, Some("boolean".to_string()), false),
                    SchemaKind::Type(Type::Object(_)) => (None, Some("object".to_string()), false),
                    _ => (None, Some("any".to_string()), false),
                }
            }
        }
    })
}
