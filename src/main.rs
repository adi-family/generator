use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use openapiv3::OpenAPI;
use std::fs;
use std::path::{Path, PathBuf};
use tera::{Tera, Context as TeraContext};

mod schema_processor;
mod operation_processor;

use schema_processor::extract_schemas;
use operation_processor::extract_operations;

#[derive(Debug, Clone, ValueEnum)]
enum Language {
    TypeScript,
    Python,
    Rust,
    Golang,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the OpenAPI specification file (YAML or JSON)
    #[arg(short, long)]
    spec: PathBuf,

    /// Target programming language
    #[arg(short, long, value_enum)]
    language: Language,

    /// Output directory for generated code
    #[arg(short, long, default_value = "generated")]
    output: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Read the OpenAPI specification
    let spec_content = fs::read_to_string(&args.spec)
        .context(format!("Failed to read spec file: {:?}", args.spec))?;

    // Parse the OpenAPI specification
    let openapi: OpenAPI = if args.spec.extension().and_then(|s| s.to_str()) == Some("json") {
        serde_json::from_str(&spec_content)?
    } else {
        serde_yaml::from_str(&spec_content)?
    };

    // Create output directory
    fs::create_dir_all(&args.output)?;

    // Extract schemas and operations
    let schemas = extract_schemas(&openapi);
    let operations = extract_operations(&openapi);

    // Generate code based on language
    match args.language {
        Language::TypeScript => generate_typescript(&openapi, &schemas, &operations, &args.output)?,
        Language::Python => generate_python(&openapi, &schemas, &operations, &args.output)?,
        Language::Golang => generate_golang(&openapi, &schemas, &operations, &args.output)?,
        Language::Rust => generate_rust(&openapi, &args.output)?,
    }

    println!("✅ Code generated successfully in {:?}", args.output);

    Ok(())
}

fn generate_typescript(
    openapi: &OpenAPI,
    schemas: &[schema_processor::ProcessedSchema],
    operations: &[operation_processor::ProcessedOperation],
    output_dir: &Path,
) -> Result<()> {
    let tera = Tera::new("templates/typescript/**/*.tera")?;
    let mut context = TeraContext::new();

    context.insert("api_title", &openapi.info.title);
    context.insert("api_version", &openapi.info.version);
    context.insert("schemas", schemas);
    context.insert("operations", operations);

    // Get base URL from servers
    let base_url = openapi
        .servers
        .first()
        .map(|s| s.url.clone())
        .unwrap_or_else(|| "http://localhost".to_string());
    context.insert("base_url", &base_url);

    let output = tera.render("client.ts.tera", &context)?;
    let output_file = output_dir.join("client.ts");
    fs::write(output_file, output)?;

    Ok(())
}

fn generate_python(
    openapi: &OpenAPI,
    schemas: &[schema_processor::ProcessedSchema],
    operations: &[operation_processor::ProcessedOperation],
    output_dir: &Path,
) -> Result<()> {
    let tera = Tera::new("templates/python/**/*.tera")?;
    let mut context = TeraContext::new();

    context.insert("api_title", &openapi.info.title);
    context.insert("api_version", &openapi.info.version);
    context.insert("schemas", schemas);
    context.insert("operations", operations);

    let base_url = openapi
        .servers
        .first()
        .map(|s| s.url.clone())
        .unwrap_or_else(|| "http://localhost".to_string());
    context.insert("base_url", &base_url);

    let output = tera.render("client.py.tera", &context)?;
    let output_file = output_dir.join("client.py");
    fs::write(output_file, output)?;

    Ok(())
}

fn generate_golang(
    openapi: &OpenAPI,
    schemas: &[schema_processor::ProcessedSchema],
    operations: &[operation_processor::ProcessedOperation],
    output_dir: &Path,
) -> Result<()> {
    let tera = Tera::new("templates/golang/**/*.tera")?;
    let mut context = TeraContext::new();

    context.insert("api_title", &openapi.info.title);
    context.insert("api_version", &openapi.info.version);
    context.insert("schemas", schemas);
    context.insert("operations", operations);

    let base_url = openapi
        .servers
        .first()
        .map(|s| s.url.clone())
        .unwrap_or_else(|| "http://localhost".to_string());
    context.insert("base_url", &base_url);

    let output = tera.render("client.go.tera", &context)?;
    let output_file = output_dir.join("client.go");
    fs::write(output_file, output)?;

    Ok(())
}

fn generate_rust(_openapi: &OpenAPI, _output_dir: &Path) -> Result<()> {
    println!("Rust generation coming soon!");
    Ok(())
}
