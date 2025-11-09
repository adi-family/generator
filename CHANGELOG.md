# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Planned
- Authentication support (Bearer, API Key, OAuth2)
- Custom error types and better error handling
- Multipart form data support
- File upload/download capabilities
- OpenAPI 3.1 support
- Full Rust client generation

## [0.1.0] - 2024-01-15

### Added
- Initial release of Generator
- OpenAPI 3.0 specification parsing using `openapiv3` crate
- TypeScript code generation with Zod validation
  - Runtime type validation with Zod schemas
  - Full type inference with `z.infer<T>`
  - tRPC-style API configuration objects
  - Type-safe client methods
- Python code generation with Pydantic models
  - Pydantic v2 models for validation
  - Full type hints with `typing` module
  - Automatic request/response serialization
  - DateTime field support
- Golang code generation
  - Native Go structs with JSON tags
  - Standard library HTTP client
  - Pointer handling for optional fields
  - Idiomatic error handling
- CLI interface with `clap`
  - `--spec` flag for OpenAPI specification path
  - `--language` flag for target language selection
  - `--output` flag for output directory
- Template-based code generation using Tera
- Schema processor for type conversion
  - Support for primitive types (string, integer, number, boolean)
  - Support for complex types (arrays, objects, enums)
  - OpenAPI reference (`$ref`) resolution
  - Optional/required field handling
- Operation processor for API endpoint extraction
  - Query parameter support
  - Path parameter support
  - Header parameter support
  - Request body handling
  - Response schema extraction
- Example PetStore OpenAPI specification
- Comprehensive README documentation
- MIT License

### Project Setup
- Rust project with Cargo
- Dependencies: openapiv3, tera, serde, clap, anyhow, indexmap
- Template system for extensible code generation
- Example directory with sample specifications

[Unreleased]: https://github.com/adi-family/generator/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/adi-family/generator/releases/tag/v0.1.0