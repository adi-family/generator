use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Unified intermediate representation (IR) for all input formats
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaIR {
    /// Normalized metadata extracted from source
    pub metadata: Metadata,

    /// Normalized schema definitions
    pub schemas: Vec<SchemaDefinition>,

    /// Normalized operation definitions
    pub operations: Vec<OperationDefinition>,

    /// Original source data (preserves all information for extensibility)
    pub original: OriginalData,
}

/// Original source data - preserves everything from the input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginalData {
    /// Format identifier (e.g., "openapi", "typescript", "graphql")
    pub format: String,

    /// Original parsed data as JSON
    pub data: JsonValue,

    /// Format-specific extensions/metadata
    #[serde(default)]
    pub extensions: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub base_url: Option<String>,

    /// Custom metadata from source (preserves non-standard fields)
    #[serde(default)]
    pub custom: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub description: Option<String>,

    /// Original schema data
    pub original: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldDefinition {
    pub name: String,
    pub type_info: TypeInfo,
    pub required: bool,
    pub description: Option<String>,

    /// Original field data
    pub original: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDefinition {
    pub id: String,
    pub method: HttpMethod,
    pub path: String,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<SchemaReference>,
    pub response: Option<SchemaReference>,
    pub description: Option<String>,

    #[serde(default)]
    pub tags: Vec<String>,

    /// Original operation data
    pub original: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
    Head,
    Options,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub location: ParameterLocation,
    pub required: bool,
    pub schema_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParameterLocation {
    Query,
    Path,
    Header,
    Cookie,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaReference {
    pub name: String,
    pub schema_type: String,
}
