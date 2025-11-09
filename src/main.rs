use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::path::PathBuf;

mod config;
mod parsers;
mod generators;
mod schema_processor;
mod operation_processor;

use config::{load_config, merge_with_cli_args};
use parsers::ParserRegistry;
use generators::GeneratorRegistry;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the OpenAPI specification file (YAML or JSON)
    #[arg(short, long)]
    spec: Option<PathBuf>,

    /// Output directory for generated code
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Path to config file (overrides default location)
    #[arg(short, long)]
    config: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let config = load_config(args.config.as_deref())?;
    let merged_config = merge_with_cli_args(config, args.spec, args.output);

    // Validate we have input
    let input_config = merged_config.input
        .ok_or_else(|| anyhow::anyhow!("No input source specified. Use --spec or configure input in config file"))?;

    println!("📖 Reading input from: {:?}", input_config.source);

    // Create parser registry
    let parser_registry = ParserRegistry::new();

    // Determine input format (explicit or auto-detect)
    let format = input_config.format.clone().unwrap_or_else(|| {
        parser_registry
            .detect_format(&input_config.source)
            .unwrap_or("openapi")
            .to_string()
    });

    println!("🔍 Detected format: {}", format);

    // Get parser
    let parser = parser_registry.get(&format)
        .ok_or_else(|| anyhow::anyhow!("Unknown input format: {}", format))?;

    // Convert serde_yaml::Value to serde_json::Value for options
    let options_json: std::collections::HashMap<String, serde_json::Value> = input_config.options.iter()
        .filter_map(|(k, v)| {
            serde_json::to_value(v).ok().map(|json_v| (k.clone(), json_v))
        })
        .collect();

    // Parse input to intermediate representation
    let schema_ir = parser.parse(&input_config.source, &options_json)
        .with_context(|| format!("Failed to parse {} input", format))?;

    println!("✅ Parsed {} schemas and {} operations",
        schema_ir.schemas.len(),
        schema_ir.operations.len()
    );

    // Create generator registry
    let generator_registry = GeneratorRegistry::new();

    // Determine output directory
    let output_dir = merged_config.output.unwrap_or_else(|| PathBuf::from("generated"));
    fs::create_dir_all(&output_dir)?;

    // Execute before hooks
    for hook in &merged_config.hooks.before_generate {
        println!("🎣 Running before hook: {}", hook);
        execute_hook(hook)?;
    }

    // Process each generation configuration
    let mut generated_count = 0;
    for gen_config in &merged_config.generations {
        if !gen_config.enabled {
            println!("⏭️  Skipping disabled generator: {}", gen_config.generator);
            continue;
        }

        println!("🔧 Generating with '{}'...", gen_config.generator);

        // Get generator
        let generator = generator_registry.get(&gen_config.generator)
            .ok_or_else(|| anyhow::anyhow!("Unknown generator: {}", gen_config.generator))?;

        // Validate config
        generator.validate_config(gen_config)?;

        // Generate code
        let output = generator.generate_from_ir(&schema_ir, gen_config)
            .with_context(|| format!("Failed to generate with '{}'", gen_config.generator))?;

        // Write to file
        let output_path = output_dir.join(&output.filename);

        fs::write(&output_path, output.content)
            .with_context(|| format!("Failed to write output file: {:?}", output_path))?;

        println!("✅ Generated: {:?}", output_path);
        generated_count += 1;
    }

    // Execute after hooks
    for hook in &merged_config.hooks.after_generate {
        println!("🎣 Running after hook: {}", hook);
        execute_hook(hook)?;
    }

    if generated_count == 0 {
        println!("⚠️  No generators were enabled. Check your configuration.");
    } else {
        println!("🎉 Successfully generated {} file(s)!", generated_count);
    }

    Ok(())
}

fn execute_hook(command: &str) -> Result<()> {
    use std::process::Command;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(["/C", command])
            .output()
    } else {
        Command::new("sh")
            .args(["-c", command])
            .output()
    }?;

    if !output.status.success() {
        anyhow::bail!(
            "Hook failed: {}\nStderr: {}",
            command,
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(())
}
