use super::schema::{Config, InputConfig};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

pub const DEFAULT_CONFIG_PATH: &str = "./.config/@adi-family/generator-config.yaml";

/// Load configuration from file or return default
pub fn load_config(custom_path: Option<&Path>) -> Result<Config> {
    let config_path = match custom_path {
        Some(path) => path.to_path_buf(),
        None => PathBuf::from(DEFAULT_CONFIG_PATH),
    };

    if config_path.exists() {
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config file: {:?}", config_path))?;

        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {:?}", config_path))?;

        Ok(config)
    } else if custom_path.is_some() {
        // Custom path specified but doesn't exist - error
        anyhow::bail!("Config file not found: {:?}", config_path);
    } else {
        // Default path doesn't exist - use built-in defaults
        Ok(Config::default())
    }
}

/// Merge config with CLI arguments (CLI takes precedence)
pub fn merge_with_cli_args(
    mut config: Config,
    spec: Option<PathBuf>,
    output: Option<PathBuf>,
) -> Config {
    // Override input source if spec provided via CLI
    if let Some(spec_path) = spec {
        if config.input.is_none() {
            config.input = Some(InputConfig {
                format: None,
                source: spec_path,
                options: Default::default(),
            });
        } else if let Some(input) = &mut config.input {
            input.source = spec_path;
        }
    }

    // Override output directory if provided via CLI
    if let Some(output_path) = output {
        config.output = Some(output_path);
    }

    config
}
