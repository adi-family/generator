use indexmap::IndexMap;
use openapiv3::{OpenAPI, ReferenceOr, Schema, SchemaKind, Type};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ProcessedSchema {
    pub name: String,
    pub properties: Vec<SchemaProperty>,
    pub required: Vec<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SchemaProperty {
    pub name: String,
    pub type_info: TypeInfo,
    pub required: bool,
    pub description: Option<String>,
    pub nullable: bool,
    pub typescript_type: String,
    pub python_type: String,
    pub golang_type: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TypeInfo {
    pub openapi_type: String,
    pub format: Option<String>,
    pub is_array: bool,
    pub array_item_type: Option<Box<TypeInfo>>,
    pub reference: Option<String>,
    pub enum_values: Option<Vec<String>>,
}

impl TypeInfo {
    pub fn to_typescript(&self) -> String {
        if self.is_array {
            if let Some(item_type) = &self.array_item_type {
                return format!("z.array({})", item_type.to_typescript_zod());
            }
            return "z.array(z.any())".to_string();
        }

        self.to_typescript_zod()
    }

    pub fn to_typescript_zod(&self) -> String {
        if let Some(ref_name) = &self.reference {
            return ref_name.clone();
        }

        if let Some(enum_vals) = &self.enum_values {
            let values: Vec<String> = enum_vals.iter().map(|v| format!("\"{}\"", v)).collect();
            return format!("z.enum([{}])", values.join(", "));
        }

        match self.openapi_type.as_str() {
            "string" => {
                if let Some(fmt) = &self.format {
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

    pub fn to_python(&self) -> String {
        if self.is_array {
            if let Some(item_type) = &self.array_item_type {
                return format!("List[{}]", item_type.to_python_type());
            }
            return "List[Any]".to_string();
        }

        self.to_python_type()
    }

    pub fn to_python_type(&self) -> String {
        if let Some(ref_name) = &self.reference {
            return ref_name.clone();
        }

        if self.enum_values.is_some() {
            return "str".to_string(); // Enums handled separately
        }

        match self.openapi_type.as_str() {
            "string" => {
                if let Some(fmt) = &self.format {
                    match fmt.as_str() {
                        "date" | "date-time" => "datetime".to_string(),
                        _ => "str".to_string(),
                    }
                } else {
                    "str".to_string()
                }
            }
            "integer" => "int".to_string(),
            "number" => "float".to_string(),
            "boolean" => "bool".to_string(),
            "object" => "Dict[str, Any]".to_string(),
            _ => "Any".to_string(),
        }
    }

    pub fn to_golang(&self) -> String {
        if self.is_array {
            if let Some(item_type) = &self.array_item_type {
                return format!("[]{}", item_type.to_golang_type());
            }
            return "[]interface{}".to_string();
        }

        self.to_golang_type()
    }

    pub fn to_golang_type(&self) -> String {
        if let Some(ref_name) = &self.reference {
            return ref_name.clone();
        }

        if self.enum_values.is_some() {
            return "string".to_string();
        }

        match self.openapi_type.as_str() {
            "string" => "string".to_string(),
            "integer" => {
                if let Some(fmt) = &self.format {
                    match fmt.as_str() {
                        "int32" => "int32".to_string(),
                        "int64" => "int64".to_string(),
                        _ => "int".to_string(),
                    }
                } else {
                    "int".to_string()
                }
            }
            "number" => {
                if let Some(fmt) = &self.format {
                    match fmt.as_str() {
                        "float" => "float32".to_string(),
                        "double" => "float64".to_string(),
                        _ => "float64".to_string(),
                    }
                } else {
                    "float64".to_string()
                }
            }
            "boolean" => "bool".to_string(),
            "object" => "map[string]interface{}".to_string(),
            _ => "interface{}".to_string(),
        }
    }
}

pub fn extract_schemas(openapi: &OpenAPI) -> Vec<ProcessedSchema> {
    let mut schemas = Vec::new();

    if let Some(components) = &openapi.components {
        for (schema_name, schema_ref) in &components.schemas {
            if let Some(schema) = process_schema_ref(schema_ref, &components.schemas) {
                schemas.push(ProcessedSchema {
                    name: schema_name.clone(),
                    properties: schema.properties,
                    required: schema.required,
                    description: schema.description,
                });
            }
        }
    }

    schemas
}

fn process_schema_ref(
    schema_ref: &ReferenceOr<Schema>,
    schemas: &IndexMap<String, ReferenceOr<Schema>>,
) -> Option<ProcessedSchema> {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            let schema_name = reference.split('/').next_back()?;
            let schema = schemas.get(schema_name)?;
            process_schema_ref(schema, schemas)
        }
        ReferenceOr::Item(schema) => Some(extract_schema_data(schema, schemas)),
    }
}

fn extract_schema_data(
    schema: &Schema,
    _schemas: &IndexMap<String, ReferenceOr<Schema>>,
) -> ProcessedSchema {
    let mut properties = Vec::new();
    let mut required = Vec::new();

    if let SchemaKind::Type(Type::Object(obj_type)) = &schema.schema_kind {
        required = obj_type.required.clone();

        for (prop_name, prop_schema_box) in &obj_type.properties {
            let is_required = required.contains(prop_name);
            let type_info = extract_type_info_from_box(prop_schema_box);

            let typescript_type = type_info.to_typescript();
            let python_type = type_info.to_python();
            let golang_type = type_info.to_golang();

            properties.push(SchemaProperty {
                name: prop_name.clone(),
                type_info,
                required: is_required,
                description: None,
                nullable: false,
                typescript_type,
                python_type,
                golang_type,
            });
        }
    }

    ProcessedSchema {
        name: String::new(),
        properties,
        required,
        description: schema.schema_data.description.clone(),
    }
}

fn extract_type_info_from_box(schema_ref: &ReferenceOr<Box<Schema>>) -> TypeInfo {
    match schema_ref {
        ReferenceOr::Reference { reference } => {
            let schema_name = reference.split('/').next_back().unwrap_or("Unknown");
            TypeInfo {
                openapi_type: "object".to_string(),
                format: None,
                is_array: false,
                array_item_type: None,
                reference: Some(schema_name.to_string()),
                enum_values: None,
            }
        }
        ReferenceOr::Item(schema_box) => extract_type_info_from_schema(schema_box.as_ref()),
    }
}


fn extract_type_info_from_schema(schema: &Schema) -> TypeInfo {
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
                Box::new(extract_type_info_from_box(items))
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
