# Extensibility Implementation Strategy for OpenAPI Generator

Based on the current codebase analysis, here are **3 approaches** for implementing extensibility, ranked from simplest to most advanced.

---

## Approach 1: Trait-Based Plugin System ⭐ **RECOMMENDED**

### Overview
Define a `Generator` trait that all language generators implement. This is native Rust, performant, and easy to extend.

### Architecture

```rust
// src/generators/mod.rs
pub trait Generator {
    /// Unique identifier for the generator (e.g., "typescript", "python")
    fn name(&self) -> &str;

    /// File extension for generated output (e.g., "ts", "py")
    fn file_extension(&self) -> &str;

    /// Generate code from OpenAPI spec
    fn generate(
        &self,
        openapi: &OpenAPI,
        schemas: &[ProcessedSchema],
        operations: &[ProcessedOperation],
        config: &GeneratorConfig,
    ) -> Result<GeneratedOutput>;

    /// Default configuration for this generator
    fn default_config(&self) -> GeneratorConfig {
        GeneratorConfig::default()
    }

    /// Validate generator-specific options
    fn validate_config(&self, config: &GeneratorConfig) -> Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct GeneratedOutput {
    pub filename: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
}
```

### Implementation Example

```rust
// src/generators/typescript.rs
use super::*;

pub struct TypeScriptGenerator;

impl Generator for TypeScriptGenerator {
    fn name(&self) -> &str {
        "typescript"
    }

    fn file_extension(&self) -> &str {
        "ts"
    }

    fn generate(
        &self,
        openapi: &OpenAPI,
        schemas: &[ProcessedSchema],
        operations: &[ProcessedOperation],
        config: &GeneratorConfig,
    ) -> Result<GeneratedOutput> {
        // Load custom template or use default
        let template_path = config.template
            .as_deref()
            .unwrap_or("templates/typescript/client.ts.tera");

        let tera = Tera::new(&format!("{}/**/*.tera", template_path))?;
        let mut context = TeraContext::new();

        // Add config options to context
        context.insert("options", &config.options);
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

        let content = tera.render("client.ts.tera", &context)?;

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content,
            metadata: HashMap::new(),
        })
    }

    fn validate_config(&self, config: &GeneratorConfig) -> Result<()> {
        // Validate TypeScript-specific options
        if let Some(class_name) = config.options.get("clientClassName") {
            if !class_name.as_str().unwrap_or("").chars().next().unwrap_or('_').is_uppercase() {
                anyhow::bail!("clientClassName must start with uppercase letter");
            }
        }
        Ok(())
    }
}
```

### Generator Registry

```rust
// src/generators/mod.rs
pub struct GeneratorRegistry {
    generators: HashMap<String, Box<dyn Generator>>,
}

impl GeneratorRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            generators: HashMap::new(),
        };

        // Register built-in generators
        registry.register(Box::new(TypeScriptGenerator));
        registry.register(Box::new(PythonGenerator));
        registry.register(Box::new(GolangGenerator));
        registry.register(Box::new(RustGenerator));

        registry
    }

    pub fn register(&mut self, generator: Box<dyn Generator>) {
        self.generators.insert(generator.name().to_string(), generator);
    }

    pub fn get(&self, name: &str) -> Option<&Box<dyn Generator>> {
        self.generators.get(name)
    }

    pub fn available_generators(&self) -> Vec<&str> {
        self.generators.keys().map(|s| s.as_str()).collect()
    }
}
```

### Updated main.rs

```rust
fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let config = config::load_config(args.config.as_deref())?;
    let merged_config = config::merge_with_cli(config, &args);

    // Parse OpenAPI spec
    let openapi = parse_openapi(&merged_config.spec)?;
    let schemas = extract_schemas(&openapi);
    let operations = extract_operations(&openapi);

    // Create generator registry
    let registry = GeneratorRegistry::new();

    // Generate for enabled languages
    for (lang_name, gen_config) in &merged_config.generators {
        if !gen_config.enabled {
            continue;
        }

        let generator = registry.get(lang_name)
            .ok_or_else(|| anyhow::anyhow!("Unknown generator: {}", lang_name))?;

        // Validate config
        generator.validate_config(gen_config)?;

        // Generate code
        let output = generator.generate(&openapi, &schemas, &operations, gen_config)?;

        // Write to file
        let output_path = merged_config.output
            .as_ref()
            .unwrap_or(&PathBuf::from("generated"))
            .join(&output.filename);

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output.content)?;

        println!("✅ Generated {} code: {:?}", lang_name, output_path);
    }

    Ok(())
}
```

### Pros
- **Native Rust** - No external dependencies, maximum performance
- **Type-safe** - Compile-time guarantees
- **Easy to add new languages** - Just implement the trait
- **Testable** - Each generator can be unit tested
- **Config-driven** - Each generator gets its own config section

### Cons
- Requires recompilation to add new generators
- All generators must be written in Rust

---

## Approach 2: External Plugin System (Dynamic Libraries) 🔧

### Overview
Load generators as external dynamic libraries (`.so`, `.dylib`, `.dll`) at runtime using `libloading`.

### Architecture

```rust
// Plugin interface (shared between core and plugins)
#[repr(C)]
pub struct PluginDeclaration {
    pub name: *const u8,
    pub name_len: usize,
    pub rustc_version: *const u8,
    pub core_version: *const u8,
    pub register: unsafe extern "C" fn(&mut dyn GeneratorRegistrar),
}

pub trait GeneratorRegistrar {
    fn register_generator(&mut self, name: &str, generator: Box<dyn Generator>);
}

// Plugin loading
pub fn load_plugin(path: &Path) -> Result<()> {
    unsafe {
        let lib = libloading::Library::new(path)?;
        let decl: libloading::Symbol<*mut PluginDeclaration> =
            lib.get(b"plugin_declaration")?;

        let decl = decl.as_ref().ok_or_else(|| anyhow!("Invalid plugin"))?;

        // Call plugin's register function
        (decl.register)(&mut registry);
    }

    Ok(())
}
```

### Example External Plugin

```rust
// external-plugins/kotlin-generator/src/lib.rs
use openapi_generator_core::*;

pub struct KotlinGenerator;

impl Generator for KotlinGenerator {
    fn name(&self) -> &str { "kotlin" }
    fn file_extension(&self) -> &str { "kt" }

    fn generate(/* ... */) -> Result<GeneratedOutput> {
        // Kotlin-specific generation logic
    }
}

#[no_mangle]
pub static plugin_declaration: PluginDeclaration = PluginDeclaration {
    name: b"kotlin\0".as_ptr(),
    name_len: 6,
    rustc_version: b"1.70.0\0".as_ptr(),
    core_version: b"0.1.0\0".as_ptr(),
    register,
};

extern "C" fn register(registrar: &mut dyn GeneratorRegistrar) {
    registrar.register_generator("kotlin", Box::new(KotlinGenerator));
}
```

### Configuration for External Plugins

```yaml
# ./.config/@adi-family/openapi-generator-config.yaml
plugins:
  - name: "kotlin"
    path: "./plugins/libkotlin_generator.so"
    enabled: true
    outputFile: "client.kt"
    options:
      packageName: "com.example.api"
```

### Pros
- **Runtime extensibility** - Add generators without recompiling
- **Language agnostic** - Plugins can be written in any language with C ABI
- **Isolated** - Plugin crashes don't crash main app
- **Third-party** - Community can create custom generators

### Cons
- Complex ABI stability issues
- Platform-specific (different libs for Linux/Mac/Windows)
- Security concerns (loading arbitrary code)
- Harder to debug

---

## Approach 3: WASM-Based Plugin System 🚀 **MOST FLEXIBLE**

### Overview
Use WebAssembly plugins via `wasmtime` for sandboxed, cross-platform extensibility.

### Architecture

```rust
// src/plugins/wasm.rs
use wasmtime::*;

pub struct WasmGenerator {
    name: String,
    engine: Engine,
    module: Module,
}

impl WasmGenerator {
    pub fn load(path: &Path) -> Result<Self> {
        let engine = Engine::default();
        let module = Module::from_file(&engine, path)?;

        Ok(Self {
            name: path.file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
            engine,
            module,
        })
    }
}

impl Generator for WasmGenerator {
    fn generate(
        &self,
        openapi: &OpenAPI,
        schemas: &[ProcessedSchema],
        operations: &[ProcessedOperation],
        config: &GeneratorConfig,
    ) -> Result<GeneratedOutput> {
        let mut store = Store::new(&self.engine, ());
        let instance = Instance::new(&mut store, &self.module, &[])?;

        // Serialize inputs to JSON
        let input_json = serde_json::json!({
            "openapi": openapi,
            "schemas": schemas,
            "operations": operations,
            "config": config,
        });

        // Call WASM function
        let generate_fn = instance.get_typed_func::<(i32, i32), i32>(&mut store, "generate")?;

        // ... memory management and result extraction

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content: output_string,
            metadata: HashMap::new(),
        })
    }
}
```

### Example WASM Plugin (Rust)

```rust
// plugins/swift-generator/src/lib.rs
#[no_mangle]
pub extern "C" fn generate(input_ptr: *const u8, input_len: usize) -> *const u8 {
    let input_bytes = unsafe { std::slice::from_raw_parts(input_ptr, input_len) };
    let input: PluginInput = serde_json::from_slice(input_bytes).unwrap();

    // Generate Swift code
    let output = format!(
        "// Swift API Client\n// Generated from {}\n\n{}",
        input.openapi.info.title,
        generate_swift_code(&input.schemas, &input.operations)
    );

    // Return JSON result
    let result = serde_json::json!({
        "filename": "client.swift",
        "content": output
    });

    let result_str = serde_json::to_string(&result).unwrap();
    result_str.as_ptr()
}

// Build: cargo build --target wasm32-wasi --release
```

### Configuration

```yaml
plugins:
  - name: "swift"
    path: "./plugins/swift_generator.wasm"
    enabled: true
    outputFile: "APIClient.swift"
    options:
      targetVersion: "5.9"
      useAsync: true
```

### Pros
- **Cross-platform** - Same WASM works everywhere
- **Sandboxed** - Secure execution environment
- **Language agnostic** - Write plugins in Rust, Go, AssemblyScript, etc.
- **Fast** - Near-native performance
- **Versionable** - Easy to distribute and update

### Cons
- Requires WASM toolchain for plugin development
- Additional runtime dependency (`wasmtime`)
- Slight performance overhead vs native
- Limited system access (by design)

---

## Comparison Matrix

| Feature | Trait-Based | Dynamic Libs | WASM |
|---------|------------|--------------|------|
| **Performance** | ⭐⭐⭐⭐⭐ Native | ⭐⭐⭐⭐⭐ Native | ⭐⭐⭐⭐ Near-native |
| **Runtime Loading** | ❌ No | ✅ Yes | ✅ Yes |
| **Cross-platform** | ✅ Yes | ❌ Platform-specific | ✅ Yes |
| **Security** | ✅ Safe | ⚠️ Unsafe | ✅ Sandboxed |
| **Ease of Development** | ⭐⭐⭐⭐⭐ Simple | ⭐⭐ Complex | ⭐⭐⭐⭐ Moderate |
| **Third-party Plugins** | ❌ Requires fork | ✅ Yes | ✅ Yes |
| **Distribution** | ⭐⭐⭐ Binary | ⭐⭐ Platform libs | ⭐⭐⭐⭐⭐ Single file |

---

## Recommended Hybrid Approach

### Phase 1: Implement Trait-Based System for Built-in Generators
- Quick to implement
- Type-safe and performant
- Easy to test and maintain

### Phase 2: Add WASM Plugin Support for External Generators
- Community can create custom generators
- Distribute as single `.wasm` files
- Secure and cross-platform

### Directory Structure

```
openapi-generator/
├── src/
│   ├── generators/
│   │   ├── mod.rs              # Generator trait + registry
│   │   ├── typescript.rs       # Built-in
│   │   ├── python.rs           # Built-in
│   │   ├── golang.rs           # Built-in
│   │   └── rust.rs             # Built-in
│   ├── plugins/
│   │   ├── mod.rs              # Plugin loader
│   │   └── wasm.rs             # WASM runtime
│   └── main.rs
├── plugins/                    # External WASM plugins
│   ├── swift_generator.wasm
│   ├── kotlin_generator.wasm
│   └── dart_generator.wasm
└── .config/@adi-family/
    └── openapi-generator-config.yaml
```

---

## Configuration File Structure

### Default Location
`./.config/@adi-family/openapi-generator-config.yaml`

### Sample Configuration

```yaml
version: "1.0"

# Source OpenAPI specification
spec: "openapi.yaml"

# Output directory for generated files
output: "generated"

# List of generation configurations - each one produces a file
# You can have multiple configs using the same or different generators
generations:
  # TypeScript types only (Zod schemas + type definitions)
  - generator: "typescript"
    outputFile: "types.ts"
    enabled: true
    # template is optional - uses built-in template if not specified
    # template: "./custom-templates/typescript/types.tera"
    options:
      zodValidation: true
      includeComments: true
      typesOnly: true  # Don't generate client class

  # TypeScript ADI HTTP compatible routes (server + client)
  - generator: "typescript_adi_http"
    outputFile: "routes.ts"
    enabled: true
    options:
      includeServer: true
      includeClient: true
      baseUrlEnvVar: "API_BASE_URL"

  # Python client with Pydantic models
  - generator: "python"
    outputFile: "client.py"
    enabled: false
    options:
      pydanticVersion: "2.0"
      asyncClient: true
      includeDocstrings: true

  # Golang native client
  - generator: "golang"
    outputFile: "client.go"
    enabled: false
    options:
      packageName: "apiclient"
      includeValidation: true

  # WASM plugin example (Phase 2)
  - generator: "swift"
    outputFile: "APIClient.swift"
    enabled: false
    plugin: "./plugins/swift_generator.wasm"  # WASM plugin path
    options:
      targetVersion: "5.9"
      useAsync: true

# Lifecycle hooks (executed once per generation run)
hooks:
  beforeGenerate: []
  afterGenerate: []

# Custom type mapping overrides (applied to all generators)
typeMapping:
  integer:
    typescript: "number"
    python: "int"
    golang: "int64"
  string:
    typescript: "string"
    python: "str"
    golang: "string"
```

### Key Configuration Changes

1. **`generations` array** - List of generation configs instead of one-per-generator
   - Each config specifies `generator`, `outputFile`, `enabled`, optional `template`
   - Allows multiple outputs from same generator (e.g., types.ts + routes.ts)

2. **Optional `template` field** - Override built-in template only when needed
   - If omitted, uses the generator's default template
   - Enables custom templates per generation config

3. **New generators:**
   - `typescript` - Types only (Zod schemas + TypeScript types, no client class)
   - `typescript_adi_http` - ADI HTTP compatible route definitions for server & client

---

## Built-in Generator Specifications

### 1. `typescript` - TypeScript Types Generator

**Purpose:** Generate Zod schemas and TypeScript type definitions only (no client class).

**Output Example:**
```typescript
// Generated TypeScript Types
// Version: 1.0.0

import { z } from 'zod';

// Pet Schema
export const PetSchema = z.object({
  id: z.number(),
  name: z.string(),
  status: z.enum(["available", "pending", "sold"]).optional(),
});

export type Pet = z.infer<typeof PetSchema>;

// User Schema
export const UserSchema = z.object({
  id: z.number(),
  username: z.string(),
  email: z.string().email(),
});

export type User = z.infer<typeof UserSchema>;
```

**Options:**
- `zodValidation: boolean` - Include Zod schemas (default: true)
- `includeComments: boolean` - Add JSDoc comments (default: true)
- `typesOnly: boolean` - Only types, no client (default: true for this generator)

**Use Case:** When you need shared types across multiple clients or want to use a different HTTP client library.

---

### 2. `typescript_adi_http` - ADI HTTP Compatible Routes Generator

**Purpose:** Generate route definitions compatible with the ADI HTTP framework for both server and client.

**What is ADI HTTP?**
A type-safe HTTP framework that uses route definitions to generate both server handlers and client methods with full TypeScript type inference.

**Output Example:**
```typescript
// Generated ADI HTTP Routes
// Version: 1.0.0

import { z } from 'zod';
import { createRoute, createRouter } from '@adi-family/http';

// Schemas
export const PetSchema = z.object({
  id: z.number(),
  name: z.string(),
  status: z.enum(["available", "pending", "sold"]).optional(),
});

export type Pet = z.infer<typeof PetSchema>;

// Route Definitions
export const routes = {
  // GET /api/pets
  listPets: createRoute({
    method: 'GET',
    path: '/api/pets',
    query: z.object({
      status: z.string().optional(),
      limit: z.coerce.number().optional(),
    }).optional(),
    response: z.array(PetSchema),
  }),

  // GET /api/pets/:id
  getPet: createRoute({
    method: 'GET',
    path: '/api/pets/:id',
    params: z.object({
      id: z.coerce.number(),
    }),
    response: PetSchema,
  }),

  // POST /api/pets
  createPet: createRoute({
    method: 'POST',
    path: '/api/pets',
    body: PetSchema.omit({ id: true }),
    response: PetSchema,
  }),

  // PUT /api/pets/:id
  updatePet: createRoute({
    method: 'PUT',
    path: '/api/pets/:id',
    params: z.object({
      id: z.coerce.number(),
    }),
    body: PetSchema.partial(),
    response: PetSchema,
  }),

  // DELETE /api/pets/:id
  deletePet: createRoute({
    method: 'DELETE',
    path: '/api/pets/:id',
    params: z.object({
      id: z.coerce.number(),
    }),
    response: z.void(),
  }),
};

// Server-side router (if includeServer: true)
export const apiRouter = createRouter(routes, {
  listPets: async (req) => {
    // Implementation provided by server
    // req.query is typed: { status?: string, limit?: number }
    // Must return: Pet[]
  },
  getPet: async (req) => {
    // req.params is typed: { id: number }
    // Must return: Pet
  },
  // ... other handlers
});

// Client-side API (if includeClient: true)
export const apiClient = createClient(routes, {
  baseUrl: process.env.API_BASE_URL || 'http://localhost:3000',
});

// Usage:
// const pets = await apiClient.listPets({ status: 'available', limit: 10 });
// const pet = await apiClient.getPet({ id: 123 });
```

**Options:**
- `includeServer: boolean` - Generate server router template (default: true)
- `includeClient: boolean` - Generate client wrapper (default: true)
- `baseUrlEnvVar: string` - Environment variable for base URL (default: "API_BASE_URL")
- `routerName: string` - Name of the router constant (default: "apiRouter")
- `clientName: string` - Name of the client constant (default: "apiClient")

**Use Case:** Full-stack TypeScript applications using the ADI HTTP framework with end-to-end type safety.

**Key Benefits:**
- **Type-safe routing** - Path params, query params, body, and response are all typed
- **Single source of truth** - Routes defined once, used by both client and server
- **Autocomplete** - Full IDE support for all API methods
- **Runtime validation** - Zod schemas validate requests/responses
- **Refactoring safety** - Changes to routes are caught at compile time

---

### 3. `python` - Python Client Generator

**Purpose:** Generate Pydantic models and Python API client.

**Output:** Single `client.py` file with Pydantic models and requests-based client.

**Options:**
- `pydanticVersion: string` - Pydantic version ("1.0" or "2.0", default: "2.0")
- `asyncClient: boolean` - Use httpx for async support (default: false)
- `includeDocstrings: boolean` - Add docstrings (default: true)

---

### 4. `golang` - Golang Client Generator

**Purpose:** Generate Go structs and HTTP client.

**Output:** Single `client.go` file with native Go types and net/http client.

**Options:**
- `packageName: string` - Go package name (default: "client")
- `includeValidation: boolean` - Add validation tags (default: true)

---

## Updated Configuration Schema (Rust)

```rust
// src/config/schema.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Config {
    pub version: String,

    #[serde(default)]
    pub spec: Option<PathBuf>,

    #[serde(default)]
    pub output: Option<PathBuf>,

    // List of generation configurations
    #[serde(default)]
    pub generations: Vec<GenerationConfig>,

    #[serde(default)]
    pub hooks: HooksConfig,

    #[serde(default)]
    pub type_mapping: Option<HashMap<String, HashMap<String, String>>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GenerationConfig {
    /// Generator name (e.g., "typescript", "typescript_adi_http", "python")
    pub generator: String,

    /// Output file path (relative to output directory)
    #[serde(rename = "outputFile")]
    pub output_file: String,

    /// Whether this generation is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Optional custom template path (if not specified, uses built-in)
    #[serde(default)]
    pub template: Option<PathBuf>,

    /// Optional WASM plugin path (for external generators)
    #[serde(default)]
    pub plugin: Option<PathBuf>,

    /// Generator-specific options
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
```

---

## Updated Main Loop

```rust
fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let config = config::load_config(args.config.as_deref())?;
    let merged_config = config::merge_with_cli(config, &args);

    // Parse OpenAPI spec
    let spec_path = merged_config.spec
        .ok_or_else(|| anyhow::anyhow!("No spec file specified"))?;
    let openapi = parse_openapi(&spec_path)?;
    let schemas = extract_schemas(&openapi);
    let operations = extract_operations(&openapi);

    // Create generator registry
    let mut registry = GeneratorRegistry::new();

    // Register built-in generators
    registry.register(Box::new(TypeScriptTypesGenerator));
    registry.register(Box::new(TypeScriptAdiHttpGenerator));
    registry.register(Box::new(PythonGenerator));
    registry.register(Box::new(GolangGenerator));

    // Execute before hooks
    for hook in &merged_config.hooks.before_generate {
        execute_hook(hook)?;
    }

    // Process each generation configuration
    for gen_config in &merged_config.generations {
        if !gen_config.enabled {
            continue;
        }

        println!("🔧 Generating with '{}'...", gen_config.generator);

        // Get generator (built-in or WASM plugin)
        let generator: Box<dyn Generator> = if let Some(plugin_path) = &gen_config.plugin {
            // Load WASM plugin
            Box::new(WasmGenerator::load(plugin_path)?)
        } else {
            // Use built-in generator
            registry.get(&gen_config.generator)
                .ok_or_else(|| anyhow::anyhow!("Unknown generator: {}", gen_config.generator))?
                .clone()
        };

        // Validate config
        generator.validate_config(gen_config)?;

        // Generate code
        let output = generator.generate(&openapi, &schemas, &operations, gen_config)?;

        // Write to file
        let output_path = merged_config.output
            .as_ref()
            .unwrap_or(&PathBuf::from("generated"))
            .join(&output.filename);

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output.content)?;

        println!("✅ Generated: {:?}", output_path);
    }

    // Execute after hooks
    for hook in &merged_config.hooks.after_generate {
        execute_hook(hook)?;
    }

    println!("🎉 All generations completed successfully!");

    Ok(())
}
```

---

## Dynamic Input System

### Overview
Instead of being limited to OpenAPI specs, the generator supports multiple input formats through a pluggable input parser system.

### Supported Input Formats

#### 1. OpenAPI Specification (Built-in)
```yaml
input:
  format: "openapi"
  source: "openapi.yaml"  # or openapi.json
```

#### 2. TypeScript Type Definitions (Planned)
```yaml
input:
  format: "typescript"
  source: "types.ts"
  options:
    extractJSDoc: true
    inferRoutes: true  # Extract route info from JSDoc @api tags
```

**Example TypeScript Input:**
```typescript
// types.ts

/**
 * Pet entity
 * @api GET /api/pets - List all pets
 * @api GET /api/pets/:id - Get pet by ID
 * @api POST /api/pets - Create new pet
 */
export interface Pet {
  /** Unique identifier */
  id: number;
  /** Pet name */
  name: string;
  /** Current status */
  status?: 'available' | 'pending' | 'sold';
}

/**
 * User entity
 * @api GET /api/users/:id - Get user
 * @api POST /api/users - Create user
 */
export interface User {
  id: number;
  username: string;
  email: string;
}
```

#### 3. GraphQL Schema (Planned)
```yaml
input:
  format: "graphql"
  source: "schema.graphql"
  options:
    convertToREST: true  # Convert queries/mutations to REST endpoints
```

#### 4. Protobuf Definitions (Planned)
```yaml
input:
  format: "protobuf"
  source: "api.proto"
```

#### 5. JSON Schema (Planned)
```yaml
input:
  format: "jsonschema"
  source: "schema.json"
  options:
    inferEndpoints: false  # Only types, no API endpoints
```

#### 6. Database Schema (Planned)
```yaml
input:
  format: "database"
  source: "postgresql://localhost/mydb"
  options:
    tables: ["users", "posts", "comments"]
    generateCRUD: true  # Auto-generate CRUD endpoints
```

---

### Input Parser Architecture

```rust
// src/parsers/mod.rs
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Unified intermediate representation (IR) for all input formats
#[derive(Debug, Clone)]
pub struct SchemaIR {
    /// Normalized metadata extracted from source
    pub metadata: Metadata,

    /// Normalized schema definitions
    pub schemas: Vec<SchemaDefinition>,

    /// Normalized operation definitions
    pub operations: Vec<OperationDefinition>,

    /// Original source data (preserves all information for extensibility)
    /// Stored as JSON for format-agnostic access
    pub original: OriginalData,
}

/// Original source data - preserves everything from the input
#[derive(Debug, Clone)]
pub struct OriginalData {
    /// Format identifier (e.g., "openapi", "typescript", "graphql")
    pub format: String,

    /// Original parsed data as JSON
    /// For OpenAPI: full OpenAPI spec
    /// For TypeScript: AST + metadata
    /// For GraphQL: parsed schema
    pub data: JsonValue,

    /// Format-specific extensions/metadata
    pub extensions: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone)]
pub struct Metadata {
    pub title: String,
    pub version: String,
    pub description: Option<String>,
    pub base_url: Option<String>,

    /// Custom metadata from source (preserves non-standard fields)
    pub custom: HashMap<String, JsonValue>,
}

#[derive(Debug, Clone)]
pub struct SchemaDefinition {
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub description: Option<String>,

    /// Original schema data (e.g., full OpenAPI schema object)
    pub original: JsonValue,
}

#[derive(Debug, Clone)]
pub struct FieldDefinition {
    pub name: String,
    pub type_info: TypeInfo,
    pub required: bool,
    pub description: Option<String>,

    /// Original field data (preserves validation rules, examples, etc.)
    pub original: JsonValue,
}

#[derive(Debug, Clone)]
pub struct OperationDefinition {
    pub id: String,
    pub method: HttpMethod,
    pub path: String,
    pub parameters: Vec<Parameter>,
    pub request_body: Option<SchemaReference>,
    pub response: Option<SchemaReference>,
    pub description: Option<String>,

    /// Tags, security, servers, and other OpenAPI-specific data
    pub tags: Vec<String>,

    /// Original operation data (e.g., full OpenAPI operation object)
    pub original: JsonValue,
}

/// Input parser trait - converts any format to unified IR
pub trait InputParser {
    /// Name of the input format (e.g., "openapi", "typescript", "graphql")
    fn format_name(&self) -> &str;

    /// File extensions this parser supports
    fn supported_extensions(&self) -> Vec<&str>;

    /// Parse input file into intermediate representation
    fn parse(&self, source: &Path, options: &HashMap<String, Value>) -> Result<SchemaIR>;

    /// Validate input file before parsing
    fn validate(&self, source: &Path) -> Result<()> {
        if !source.exists() {
            anyhow::bail!("Input file not found: {:?}", source);
        }
        Ok(())
    }
}
```

---

### Built-in Input Parsers

#### OpenAPI Parser
```rust
// src/parsers/openapi.rs

pub struct OpenApiParser;

impl InputParser for OpenApiParser {
    fn format_name(&self) -> &str {
        "openapi"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["yaml", "yml", "json"]
    }

    fn parse(&self, source: &Path, _options: &HashMap<String, Value>) -> Result<SchemaIR> {
        let content = fs::read_to_string(source)?;

        let openapi: OpenAPI = if source.extension() == Some("json".as_ref()) {
            serde_json::from_str(&content)?
        } else {
            serde_yaml::from_str(&content)?
        };

        // Serialize OpenAPI spec to JSON for storage in original data
        let original_json = serde_json::to_value(&openapi)?;

        // Extract custom metadata (x- extensions)
        let mut custom_metadata = HashMap::new();
        if let Some(extensions) = &openapi.extensions {
            for (key, value) in extensions {
                custom_metadata.insert(key.clone(), value.clone());
            }
        }

        // Convert OpenAPI to SchemaIR
        Ok(SchemaIR {
            metadata: Metadata {
                title: openapi.info.title.clone(),
                version: openapi.info.version.clone(),
                description: openapi.info.description.clone(),
                base_url: openapi.servers.first().map(|s| s.url.clone()),
                custom: custom_metadata,
            },
            schemas: convert_schemas(&openapi),
            operations: convert_operations(&openapi),
            original: OriginalData {
                format: "openapi".to_string(),
                data: original_json,
                extensions: extract_extensions(&openapi),
            },
        })
    }
}

fn convert_schemas(openapi: &OpenAPI) -> Vec<SchemaDefinition> {
    let mut schemas = Vec::new();

    if let Some(components) = &openapi.components {
        for (schema_name, schema_ref) in &components.schemas {
            if let ReferenceOr::Item(schema) = schema_ref {
                // Serialize original schema to JSON
                let original_json = serde_json::to_value(schema).unwrap_or(JsonValue::Null);

                schemas.push(SchemaDefinition {
                    name: schema_name.clone(),
                    fields: extract_fields(schema),
                    description: schema.schema_data.description.clone(),
                    original: original_json,
                });
            }
        }
    }

    schemas
}

fn extract_fields(schema: &Schema) -> Vec<FieldDefinition> {
    let mut fields = Vec::new();

    if let SchemaKind::Type(Type::Object(obj_type)) = &schema.schema_kind {
        let required = &obj_type.required;

        for (field_name, field_schema_ref) in &obj_type.properties {
            let is_required = required.contains(field_name);

            // Get the actual schema (resolve references or use item)
            let field_schema = match field_schema_ref {
                ReferenceOr::Item(schema_box) => schema_box.as_ref(),
                ReferenceOr::Reference { .. } => {
                    // For references, store the reference info
                    continue; // Handle separately
                }
            };

            // Serialize original field schema
            let original_json = serde_json::to_value(field_schema).unwrap_or(JsonValue::Null);

            fields.push(FieldDefinition {
                name: field_name.clone(),
                type_info: extract_type_info(field_schema),
                required: is_required,
                description: field_schema.schema_data.description.clone(),
                original: original_json, // Preserves validation, examples, etc.
            });
        }
    }

    fields
}

fn extract_extensions(openapi: &OpenAPI) -> HashMap<String, JsonValue> {
    let mut extensions = HashMap::new();

    // Extract OpenAPI-specific extensions that might be useful
    if let Some(ext) = &openapi.extensions {
        for (key, value) in ext {
            extensions.insert(format!("openapi.{}", key), value.clone());
        }
    }

    extensions
}
```

#### TypeScript Parser (Future)
```rust
// src/parsers/typescript.rs
use swc_ecma_parser::{Parser, StringInput, Syntax};
use swc_ecma_ast::*;

pub struct TypeScriptParser;

impl InputParser for TypeScriptParser {
    fn format_name(&self) -> &str {
        "typescript"
    }

    fn supported_extensions(&self) -> Vec<&str> {
        vec!["ts", "tsx", "d.ts"]
    }

    fn parse(&self, source: &Path, options: &HashMap<String, Value>) -> Result<SchemaIR> {
        let content = fs::read_to_string(source)?;

        // Parse TypeScript using SWC
        let module = parse_typescript(&content)?;

        // Extract interfaces, types, and JSDoc comments
        let schemas = extract_type_definitions(&module)?;

        // Extract API routes from JSDoc @api tags
        let operations = if options.get("inferRoutes").and_then(|v| v.as_bool()).unwrap_or(false) {
            extract_routes_from_jsdoc(&module)?
        } else {
            vec![]
        };

        Ok(SchemaIR {
            metadata: Metadata {
                title: "TypeScript API".to_string(),
                version: "1.0.0".to_string(),
                description: None,
                base_url: None,
            },
            schemas,
            operations,
        })
    }
}

fn extract_type_definitions(module: &Module) -> Result<Vec<SchemaDefinition>> {
    // Walk AST and extract interface/type definitions
    // Convert TypeScript types to our TypeInfo representation
    // ...
}

fn extract_routes_from_jsdoc(module: &Module) -> Result<Vec<OperationDefinition>> {
    // Parse JSDoc comments like:
    // @api GET /api/users/:id - Get user by ID
    // @apiParam {number} id - User ID
    // @apiResponse {User} - User object
    // ...
}
```

---

### Updated Configuration

```yaml
version: "1.0"

# Input configuration - now dynamic!
input:
  format: "openapi"  # or "typescript", "graphql", "protobuf", etc.
  source: "openapi.yaml"
  options:
    # Format-specific options
    extractJSDoc: true  # for TypeScript
    convertToREST: true  # for GraphQL

# Alternative: Multiple inputs (Phase 2)
# inputs:
#   - format: "openapi"
#     source: "api-v1.yaml"
#   - format: "typescript"
#     source: "types.ts"
#     options:
#       inferRoutes: true

# Output directory
output: "generated"

# Generation configurations
generations:
  - generator: "typescript"
    outputFile: "types.ts"
    enabled: true

  - generator: "typescript_adi_http"
    outputFile: "routes.ts"
    enabled: true
    options:
      includeServer: true
      includeClient: true
```

---

### Parser Registry

```rust
// src/parsers/mod.rs

pub struct ParserRegistry {
    parsers: HashMap<String, Box<dyn InputParser>>,
}

impl ParserRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            parsers: HashMap::new(),
        };

        // Register built-in parsers
        registry.register(Box::new(OpenApiParser));

        // Future parsers (Phase 2)
        // registry.register(Box::new(TypeScriptParser));
        // registry.register(Box::new(GraphQLParser));
        // registry.register(Box::new(ProtobufParser));

        registry
    }

    pub fn register(&mut self, parser: Box<dyn InputParser>) {
        self.parsers.insert(parser.format_name().to_string(), parser);
    }

    pub fn get(&self, format: &str) -> Option<&Box<dyn InputParser>> {
        self.parsers.get(format)
    }

    /// Auto-detect format from file extension
    pub fn detect_format(&self, path: &Path) -> Option<&str> {
        let ext = path.extension()?.to_str()?;

        for parser in self.parsers.values() {
            if parser.supported_extensions().contains(&ext) {
                return Some(parser.format_name());
            }
        }

        None
    }
}
```

---

### Updated Main Flow

```rust
fn main() -> Result<()> {
    let args = Args::parse();

    // Load configuration
    let config = config::load_config(args.config.as_deref())?;
    let merged_config = config::merge_with_cli(config, &args);

    // Create parser registry
    let parser_registry = ParserRegistry::new();

    // Parse input (auto-detect or use specified format)
    let input_config = merged_config.input
        .ok_or_else(|| anyhow::anyhow!("No input specified"))?;

    let format = input_config.format.unwrap_or_else(|| {
        parser_registry
            .detect_format(&input_config.source)
            .unwrap_or("openapi")
            .to_string()
    });

    let parser = parser_registry.get(&format)
        .ok_or_else(|| anyhow::anyhow!("Unknown input format: {}", format))?;

    // Parse input to intermediate representation
    let schema_ir = parser.parse(&input_config.source, &input_config.options)?;

    // Create generator registry
    let generator_registry = GeneratorRegistry::new();

    // Execute before hooks
    execute_hooks(&merged_config.hooks.before_generate)?;

    // Process each generation configuration
    for gen_config in &merged_config.generations {
        if !gen_config.enabled {
            continue;
        }

        println!("🔧 Generating with '{}'...", gen_config.generator);

        let generator = get_generator(&generator_registry, gen_config)?;

        // Generate code from IR (not tied to OpenAPI anymore!)
        let output = generator.generate_from_ir(&schema_ir, gen_config)?;

        let output_path = merged_config.output
            .as_ref()
            .unwrap_or(&PathBuf::from("generated"))
            .join(&output.filename);

        fs::create_dir_all(output_path.parent().unwrap())?;
        fs::write(&output_path, output.content)?;

        println!("✅ Generated: {:?}", output_path);
    }

    execute_hooks(&merged_config.hooks.after_generate)?;

    println!("🎉 All generations completed successfully!");

    Ok(())
}
```

---

### Updated Generator Trait

```rust
// src/generators/mod.rs

pub trait Generator {
    fn name(&self) -> &str;
    fn file_extension(&self) -> &str;

    /// Generate from intermediate representation (format-agnostic)
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput>;

    fn validate_config(&self, config: &GenerationConfig) -> Result<()> {
        Ok(())
    }
}
```

---

### Using Original Data for Extensibility

#### Why Preserve Original Data?

The `original` field in SchemaIR and its nested structures enables:

1. **Zero information loss** - All source data preserved, even format-specific features
2. **Advanced generators** - Access to full spec for specialized features
3. **Custom extensions** - Support vendor extensions (e.g., `x-custom` in OpenAPI)
4. **Format-specific optimizations** - Generators can use format-specific features
5. **Debugging** - Trace generated code back to source

#### Example 1: Accessing OpenAPI Validation Rules

```rust
// In a generator that needs OpenAPI validation constraints
impl Generator for TypeScriptZodGenerator {
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        let mut output = String::new();

        for schema in &schema_ir.schemas {
            for field in &schema.fields {
                // Access normalized data
                let field_name = &field.name;
                let base_type = &field.type_info;

                // Access original OpenAPI schema for validation rules
                if let Some(original_field) = field.original.as_object() {
                    let mut zod_validators = vec![];

                    // Extract min/max from original OpenAPI schema
                    if let Some(min) = original_field.get("minimum") {
                        zod_validators.push(format!(".min({})", min));
                    }
                    if let Some(max) = original_field.get("maximum") {
                        zod_validators.push(format!(".max({})", max));
                    }

                    // Extract pattern (regex)
                    if let Some(pattern) = original_field.get("pattern").and_then(|p| p.as_str()) {
                        zod_validators.push(format!(".regex(/{}/)", pattern));
                    }

                    // Extract examples for documentation
                    if let Some(example) = original_field.get("example") {
                        output.push_str(&format!("  // Example: {}\n", example));
                    }

                    // Generate Zod schema with validation
                    output.push_str(&format!(
                        "  {}: z.{}(){},\n",
                        field_name,
                        base_type.to_zod_type(),
                        zod_validators.join("")
                    ));
                }
            }
        }

        Ok(GeneratedOutput {
            filename: config.output_file.clone(),
            content: output,
            metadata: HashMap::new(),
        })
    }
}
```

**Generated Output:**
```typescript
// Example: 42
age: z.number().min(0).max(120),
// Example: "john@example.com"
email: z.string().regex(/^[^@]+@[^@]+$/),
```

#### Example 2: Using OpenAPI Extensions (x- fields)

```yaml
# OpenAPI spec with custom extensions
components:
  schemas:
    User:
      type: object
      x-table-name: users  # Custom extension
      x-audit: true        # Custom extension
      properties:
        id:
          type: integer
          x-primary-key: true  # Custom extension
        email:
          type: string
          x-searchable: true   # Custom extension
```

```rust
// Generator that uses custom extensions
impl Generator for DatabaseMigrationGenerator {
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        let mut sql = String::new();

        for schema in &schema_ir.schemas {
            // Access custom extension from original data
            let table_name = schema.original
                .get("x-table-name")
                .and_then(|v| v.as_str())
                .unwrap_or(&schema.name.to_lowercase());

            sql.push_str(&format!("CREATE TABLE {} (\n", table_name));

            for field in &schema.fields {
                let sql_type = map_to_sql_type(&field.type_info);

                // Check if this field is the primary key
                let is_pk = field.original
                    .get("x-primary-key")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let constraints = if is_pk { " PRIMARY KEY" } else { "" };

                sql.push_str(&format!(
                    "  {} {}{},\n",
                    field.name, sql_type, constraints
                ));

                // Check if field should be indexed
                let is_searchable = field.original
                    .get("x-searchable")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                if is_searchable {
                    sql.push_str(&format!(
                        "CREATE INDEX idx_{}_{} ON {} ({});\n",
                        table_name, field.name, table_name, field.name
                    ));
                }
            }

            sql.push_str(");\n\n");

            // Check if audit triggers needed
            let needs_audit = schema.original
                .get("x-audit")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            if needs_audit {
                sql.push_str(&format!(
                    "CREATE TRIGGER audit_{} BEFORE UPDATE ON {}...\n\n",
                    table_name, table_name
                ));
            }
        }

        Ok(GeneratedOutput {
            filename: "migration.sql".to_string(),
            content: sql,
            metadata: HashMap::new(),
        })
    }
}
```

**Generated Output:**
```sql
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  email TEXT,
);
CREATE INDEX idx_users_email ON users (email);

CREATE TRIGGER audit_users BEFORE UPDATE ON users...
```

#### Example 3: Format Detection and Conditional Logic

```rust
impl Generator for UniversalDocGenerator {
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        let mut doc = String::new();

        // Different documentation styles based on input format
        match schema_ir.original.format.as_str() {
            "openapi" => {
                // Use OpenAPI-specific documentation features
                if let Some(openapi_data) = schema_ir.original.data.as_object() {
                    // Access OpenAPI security schemes
                    if let Some(security) = openapi_data
                        .get("components")
                        .and_then(|c| c.get("securitySchemes"))
                    {
                        doc.push_str("## Authentication\n\n");
                        doc.push_str(&format!("{}\n\n", serde_json::to_string_pretty(security)?));
                    }

                    // Access OpenAPI servers
                    if let Some(servers) = openapi_data.get("servers") {
                        doc.push_str("## Base URLs\n\n");
                        for server in servers.as_array().unwrap_or(&vec![]) {
                            if let Some(url) = server.get("url") {
                                doc.push_str(&format!("- {}\n", url));
                            }
                        }
                        doc.push_str("\n");
                    }
                }
            }
            "typescript" => {
                // Use TypeScript-specific documentation
                doc.push_str("## TypeScript Types\n\n");
                doc.push_str("These types were extracted from TypeScript source.\n\n");
            }
            "graphql" => {
                // Use GraphQL-specific documentation
                doc.push_str("## GraphQL Schema\n\n");
            }
            _ => {}
        }

        // Generate common documentation for all formats
        for schema in &schema_ir.schemas {
            doc.push_str(&format!("### {}\n\n", schema.name));
            // ...
        }

        Ok(GeneratedOutput {
            filename: "API_DOCS.md".to_string(),
            content: doc,
            metadata: HashMap::new(),
        })
    }
}
```

#### Example 4: Accessing Full OpenAPI Spec for Advanced Features

```rust
// Generator that needs access to OpenAPI-specific features
impl Generator for OpenApiValidatorGenerator {
    fn generate_from_ir(
        &self,
        schema_ir: &SchemaIR,
        config: &GenerationConfig,
    ) -> Result<GeneratedOutput> {
        // Deserialize original OpenAPI spec
        let openapi: OpenAPI = if schema_ir.original.format == "openapi" {
            serde_json::from_value(schema_ir.original.data.clone())?
        } else {
            anyhow::bail!("This generator only works with OpenAPI input");
        };

        // Now we have full access to all OpenAPI features
        let mut validators = String::new();

        // Access security schemes (not in normalized IR)
        if let Some(components) = &openapi.components {
            if let Some(security_schemes) = &components.security_schemes {
                validators.push_str("// Security validators\n");
                for (name, scheme) in security_schemes {
                    validators.push_str(&format!(
                        "export const validate{} = (req) => {{ /* ... */ }};\n",
                        name
                    ));
                }
            }

            // Access callbacks (not in normalized IR)
            if let Some(callbacks) = &components.callbacks {
                validators.push_str("// Callback validators\n");
                // Generate callback validators...
            }
        }

        // Access global security requirements
        for security_req in &openapi.security {
            // Generate middleware for global security...
        }

        Ok(GeneratedOutput {
            filename: "validators.ts".to_string(),
            content: validators,
            metadata: HashMap::new(),
        })
    }
}
```

---

### Benefits of Dynamic Input System

1. **Format Agnostic** - Support any schema format (OpenAPI, TypeScript, GraphQL, etc.)
2. **Unified IR** - Single intermediate representation simplifies generator implementation
3. **Extensible** - Easy to add new input parsers via trait
4. **Auto-detection** - Can infer format from file extension
5. **Future-proof** - Add database schema, Protobuf, etc. without changing generators
6. **Zero Information Loss** - Original data preserved for advanced use cases
7. **Custom Extensions** - Support vendor-specific features (x- fields, custom annotations)

---

### Use Cases

#### Use Case 1: TypeScript → OpenAPI
```yaml
input:
  format: "typescript"
  source: "types.ts"

generations:
  - generator: "openapi"
    outputFile: "openapi.yaml"
```

#### Use Case 2: OpenAPI → Multiple Languages
```yaml
input:
  format: "openapi"
  source: "api.yaml"

generations:
  - generator: "typescript"
    outputFile: "types.ts"
  - generator: "python"
    outputFile: "client.py"
  - generator: "golang"
    outputFile: "client.go"
```

#### Use Case 3: GraphQL → REST Client
```yaml
input:
  format: "graphql"
  source: "schema.graphql"
  options:
    convertToREST: true

generations:
  - generator: "typescript_adi_http"
    outputFile: "rest-routes.ts"
```

#### Use Case 4: Database → API + Types
```yaml
input:
  format: "database"
  source: "postgresql://localhost/mydb"
  options:
    tables: ["users", "posts"]
    generateCRUD: true

generations:
  - generator: "typescript_adi_http"
    outputFile: "api-routes.ts"
  - generator: "typescript"
    outputFile: "db-types.ts"
```

---

### Implementation Phases

#### Phase 1: Core + OpenAPI Parser ✅
- Intermediate representation (SchemaIR)
- Parser trait + registry
- OpenAPI parser (migrate existing code)
- Update generators to use IR

#### Phase 2: TypeScript Parser
- Add SWC dependency for TypeScript parsing
- Extract interfaces/types from AST
- Parse JSDoc annotations for API routes
- Support .d.ts files

#### Phase 3: Additional Parsers
- GraphQL schema parser
- JSON Schema parser
- Protobuf parser
- Database introspection

---

## Next Steps

### Implementation Roadmap

1. **Phase 1: Core Configuration System**
   - [ ] Create `src/config/` module structure
   - [ ] Define configuration schema structs
   - [ ] Implement config loader with default path support
   - [ ] Add `--config` CLI flag
   - [ ] Implement CLI override merging logic
   - [ ] Add config validation

2. **Phase 2: Trait-Based Generator System**
   - [ ] Create `Generator` trait
   - [ ] Refactor TypeScript generator
   - [ ] Refactor Python generator
   - [ ] Refactor Golang generator
   - [ ] Implement generator registry
   - [ ] Support multiple language generation

3. **Phase 3: WASM Plugin Support** (Future)
   - [ ] Add `wasmtime` dependency
   - [ ] Implement WASM plugin loader
   - [ ] Define plugin interface/protocol
   - [ ] Create example WASM plugin
   - [ ] Documentation for plugin development

---

## Benefits

### For Users
- **Flexible configuration** - YAML-based setup with sensible defaults
- **Multi-language support** - Generate for multiple languages in one run
- **Customizable** - Override templates, type mappings, and options
- **Backward compatible** - Pure CLI usage still works

### For Developers
- **Type-safe** - Rust's type system ensures correctness
- **Testable** - Each component can be unit tested
- **Maintainable** - Clear separation of concerns
- **Extensible** - Easy to add new generators

### For Community
- **Plugin ecosystem** - WASM-based plugins (Phase 2)
- **Language agnostic** - Write plugins in any language
- **Secure** - Sandboxed execution
- **Portable** - Single WASM file distribution
