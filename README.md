# Generator

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![GitHub](https://img.shields.io/badge/github-adi--family%2Fgenerator-blue)](https://github.com/adi-family/generator)

A powerful Rust-based code generator that transforms OpenAPI 3.0 specifications into type-safe client libraries with native runtime validation for TypeScript (Zod), Python (Pydantic), and Golang.

## Why Generator?

- **🎯 Type Safety First**: Generates fully type-safe clients with compile-time and runtime validation
- **🚀 Native Validation**: Uses idiomatic validation libraries (Zod, Pydantic) for each language
- **📦 Zero-Config**: Works out of the box with sensible defaults
- **🔧 OpenAPI 3.0**: Full support for modern OpenAPI specifications
- **⚡ Fast**: Written in Rust for maximum performance
- **🎨 Customizable**: Template-based generation with Tera templates

## Features

### TypeScript Generation
- ✅ **Zod schemas** for runtime validation
- ✅ **Full type inference** with `z.infer<T>`
- ✅ **tRPC-style API configs** for composable API definitions
- ✅ **Type-safe client methods** with IntelliSense support
- ✅ **Query, path, header, and body parameters**
- ✅ **Optional/required field handling**
- ✅ **Array and nested object support**
- ✅ **Enum types**

### Python Generation
- ✅ **Pydantic models** for validation and serialization
- ✅ **Type hints** with the `typing` module
- ✅ **Automatic request/response serialization**
- ✅ **DateTime field support**
- ✅ **Optional/required field handling**
- ✅ **Nested models and arrays**

### Golang Generation
- ✅ **Native Go structs** with JSON tags
- ✅ **Type-safe API methods**
- ✅ **Pointer handling** for optional fields
- ✅ **Standard library HTTP client**
- ✅ **Query parameter encoding**
- ✅ **Idiomatic Go error handling**

## Installation

### From Source

```bash
git clone https://github.com/adi-family/generator.git
cd generator
cargo build --release
```

The binary will be available at `./target/release/generator`.

### Using Cargo

```bash
cargo install --path .
```

## Quick Start

Generate a TypeScript client from the example PetStore API:

```bash
generator \
  --spec examples/petstore.yaml \
  --language type-script \
  --output generated/typescript
```

This creates a fully type-safe TypeScript client with Zod validation:

```typescript
import { ApiClient, Pet, PetSchema } from './generated/typescript';

const client = new ApiClient({ baseUrl: 'https://api.example.com' });

// Fully typed with IntelliSense
const pets = await client.listPets({ limit: 10 });
// pets: Pet[]

// Runtime validation with Zod
const pet = PetSchema.parse(response);
```

## Usage

### Command Line Interface

```bash
generator [OPTIONS] --spec <PATH> --language <LANG>
```

**Options:**

- `-s, --spec <PATH>` - Path to OpenAPI specification (YAML or JSON) **[required]**
- `-l, --language <LANG>` - Target language: `type-script`, `python`, `golang`, `rust` **[required]**
- `-o, --output <DIR>` - Output directory for generated code (default: `generated`)
- `-h, --help` - Print help information
- `-V, --version` - Print version information

### Examples

**TypeScript with Zod:**
```bash
generator -s api.yaml -l type-script -o ./client
```

**Python with Pydantic:**
```bash
generator -s api.yaml -l python -o ./sdk
```

**Golang:**
```bash
generator -s api.yaml -l golang -o ./pkg/client
```

## Generated Code Examples

### TypeScript

```typescript
// Zod schemas with full type inference
export const PetSchema = z.object({
  id: z.string(),
  name: z.string(),
  tag: z.string().optional(),
});

export type Pet = z.infer<typeof PetSchema>;

// API configuration objects (tRPC-style)
export const listPetsConfig = {
  method: 'GET',
  path: '/pets',
  parameters: {
    query: {
      schema: z.object({
        limit: z.coerce.number().optional(),
      }).optional()
    },
  },
  response: {
    schema: z.array(PetSchema)
  }
} as const;

// Type-safe API client
const client = new ApiClient({ baseUrl: 'https://api.example.com' });
const pets = await client.listPets({ limit: 10 }); // Pet[]
```

### Python

```python
from pydantic import BaseModel, Field
from typing import Optional, List

# Pydantic models
class Pet(BaseModel):
    id: str = Field(...)
    name: str = Field(...)
    tag: Optional[str] = None

# Type-safe API client
client = ApiClient(ApiClientConfig(base_url='https://api.example.com'))
pets: List[Pet] = client.list_pets(limit=10)
```

### Golang

```go
// Native structs
type Pet struct {
    Id   string `json:"id"`
    Name string `json:"name"`
    Tag  string `json:"tag,omitempty"`
}

// Type-safe API client
client := NewApiClient(&ApiClientConfig{
    BaseURL: "https://api.example.com",
})
limit := 10
pets, err := client.ListPets(&limit)  // []Pet, error
if err != nil {
    log.Fatal(err)
}
```

## Architecture

### Core Components

```
┌─────────────────────────────────────────────────────┐
│                 OpenAPI Spec (YAML/JSON)            │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│            Schema Processor                         │
│  • Extract schema definitions                       │
│  • Convert OpenAPI types to target types           │
│  • Handle references, enums, arrays                 │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│          Operation Processor                        │
│  • Extract API paths and operations                 │
│  • Process parameters (query, path, header)         │
│  • Handle request/response bodies                   │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│           Template Engine (Tera)                    │
│  • Language-specific templates                      │
│  • Context building with schemas + operations       │
│  • Code generation and formatting                   │
└──────────────────┬──────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────┐
│         Generated Code (TS/Python/Go)               │
└─────────────────────────────────────────────────────┘
```

### Type Mapping

| OpenAPI Type       | TypeScript (Zod)            | Python (Pydantic) | Golang      |
|--------------------|----------------------------|-------------------|-------------|
| `string`           | `z.string()`               | `str`             | `string`    |
| `integer`          | `z.number()`               | `int`             | `int`       |
| `number`           | `z.number()`               | `float`           | `float64`   |
| `boolean`          | `z.boolean()`              | `bool`            | `bool`      |
| `array`            | `z.array(T)`               | `List[T]`         | `[]T`       |
| `object`           | `z.object({...})`          | `BaseModel`       | `struct`    |
| `string(date)`     | `z.date().or(z.string())`  | `datetime`        | `string`    |
| `string(date-time)`| `z.date().or(z.string())`  | `datetime`        | `string`    |
| `enum`             | `z.enum([...])`            | `Literal[...]`    | `string`    |

## Project Structure

```
generator/
├── src/
│   ├── main.rs                 # CLI and generator orchestration
│   ├── schema_processor.rs     # OpenAPI schema extraction
│   ├── operation_processor.rs  # API operation extraction
│   └── config/
│       └── loader.rs           # Configuration loading
├── templates/
│   ├── typescript/
│   │   └── client.ts.tera      # TypeScript + Zod template
│   ├── python/
│   │   └── client.py.tera      # Python + Pydantic template
│   └── golang/
│       └── client.go.tera      # Golang template
├── examples/
│   └── petstore.yaml           # Example OpenAPI spec
└── Cargo.toml
```

## Dependencies

### Runtime Dependencies (Rust)
- `openapiv3` - OpenAPI 3.0 parsing
- `tera` - Template engine
- `serde` / `serde_json` / `serde_yaml` - Serialization
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `indexmap` - Ordered maps

### Generated Code Dependencies

**TypeScript:**
```json
{
  "dependencies": {
    "zod": "^3.22.0"
  }
}
```

**Python:**
```txt
pydantic>=2.0.0
requests>=2.31.0
```

**Golang:**
- Standard library only (no external dependencies)

## Why OpenAPI v3?

We use the `openapiv3` crate for parsing because:

- ✅ **Simplicity**: Clean, straightforward data structures
- ✅ **Stability**: Mature and well-tested (v2.0+)
- ✅ **Minimal dependencies**: Lightweight design
- ✅ **Perfect for code generation**: All the features we need, nothing we don't

**Alternatives considered:**
- `oapi`: Better for validation/linting workflows, but adds complexity
- `oas3`/`openapiv3_1`: Required for OpenAPI 3.1.x, but we target 3.0.x

## Limitations & Known Issues

- Currently supports OpenAPI 3.0 (not 3.1)
- Authentication schemes not yet implemented
- Basic error response handling
- No multipart form data support
- File upload/download not supported

## Roadmap

### v0.2.0
- [ ] Bearer token authentication
- [ ] API Key authentication
- [ ] Custom error types
- [ ] Better error messages

### v0.3.0
- [ ] Multipart form data
- [ ] File upload/download
- [ ] OAuth2 support

### v1.0.0
- [ ] OpenAPI 3.1 support
- [ ] Webhook definitions
- [ ] Full Rust client generation
- [ ] Comprehensive test suite

### Future
- [ ] Additional languages (Java, C#, Ruby)
- [ ] Custom template support
- [ ] Plugin system
- [ ] Configuration file support

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

### Development Setup

```bash
# Clone the repository
git clone https://github.com/adi-family/generator.git
cd generator

# Build the project
cargo build

# Run tests
cargo test

# Generate example code
cargo run -- -s examples/petstore.yaml -l type-script -o generated/test
```

### Adding a New Language

1. Create a template in `templates/<language>/client.<ext>.tera`
2. Add type mapping logic in `src/schema_processor.rs`
3. Update the CLI language enum in `src/main.rs`
4. Add example output to the README

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Support

- 📖 [Documentation](https://github.com/adi-family/generator)
- 🐛 [Issue Tracker](https://github.com/adi-family/generator/issues)
- 💬 [Discussions](https://github.com/adi-family/generator/discussions)

## Acknowledgments

Built with ❤️ by the ADI Family team.

---

**Star ⭐ this repository if you find it useful!**