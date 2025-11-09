use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub version: String,

    #[serde(default)]
    pub input: Option<InputConfig>,

    #[serde(default)]
    pub output: Option<PathBuf>,

    #[serde(default)]
    pub generations: Vec<GenerationConfig>,

    #[serde(default)]
    pub hooks: HooksConfig,

    #[serde(default)]
    pub type_mapping: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct InputConfig {
    #[serde(default)]
    pub format: Option<String>,

    pub source: PathBuf,

    #[serde(default)]
    pub options: HashMap<String, serde_yaml::Value>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GenerationConfig {
    pub generator: String,

    #[serde(rename = "outputFile")]
    pub output_file: String,

    #[serde(default = "default_true")]
    pub enabled: bool,

    #[serde(default)]
    pub template: Option<PathBuf>,

    #[serde(default)]
    pub plugin: Option<PathBuf>,

    #[serde(default)]
    pub options: HashMap<String, serde_yaml::Value>,
}

fn default_true() -> bool {
    true
}

#[derive(Deserialize, Serialize, Debug, Clone, Default)]
pub struct HooksConfig {
    #[serde(rename = "beforeGenerate", default)]
    pub before_generate: Vec<String>,

    #[serde(rename = "afterGenerate", default)]
    pub after_generate: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            input: None,
            output: Some(PathBuf::from("generated")),
            generations: vec![],
            hooks: HooksConfig::default(),
            type_mapping: None,
        }
    }
}
