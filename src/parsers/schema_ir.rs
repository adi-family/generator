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
