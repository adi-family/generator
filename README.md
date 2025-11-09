# Code Generator

A Rust-based code generator that creates type-safe client libraries for TypeScript, Python, and Golang with native types from OpenAPI 3.0 specifications.

## Features

### TypeScript Generation
- ✅ Zod schemas for runtime validation
- ✅ Full type inference with `z.infer`
- ✅ API configuration objects (similar to tRPC style)
- ✅ Type-safe API client methods
- ✅ Query, path, and body parameter support
- ✅ Proper handling of optional/required fields
- ✅ Array response types

### Python Generation
- ✅ Pydantic models for validation
- ✅ Full type hints with `typing` module
- ✅ Type-safe API client methods
- ✅ Automatic request/response serialization
- ✅ Support for datetime fields
- ✅ Optional/required field handling

### Golang Generation
- ✅ Native Go structs with JSON tags
- ✅ Type-safe API methods
- ✅ Proper pointer handling for optional fields
- ✅ Standard library HTTP client
- ✅ Query parameter encoding
- ✅ Error handling

## Installation

```bash
cargo build --release
```

## Usage

Generate client code from an OpenAPI specification:

```bash
# TypeScript with Zod
./target/release/generator \
  --spec examples/petstore.yaml \
  --language type-script \
  --output generated/typescript

# Python with Pydantic
./target/release/generator \
  --spec examples/petstore.yaml \
  --language python \
  --output generated/python

# Golang
./target/release/generator \
  --spec examples/petstore.yaml \
  --language golang \
  --output generated/golang
```

Options:
- `-s, --spec <PATH>`: Path to the OpenAPI specification file (YAML or JSON)
- `-l, --language <LANG>`: Target programming language (type-script, python, golang, rust)
- `-o, --output <DIR>`: Output directory for generated code (default: "generated")

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

// API configuration objects
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
# Pydantic models
class Pet(BaseModel):
    id: str = Field(...)
    name: str = Field(...)
    tag: Optional[str] = None

# Type-safe API client
client = ApiClient(ApiClientConfig(base_url='https://api.example.com'))
pets = client.listPets(limit=10)  # List[Pet]
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
pets, err := client.ListPets(&limit)  // []Pet
```

## Architecture

### Core Components

1. **Schema Processor** (`src/schema_processor.rs`)
   - Extracts OpenAPI schema definitions
   - Converts OpenAPI types to target language types
   - Handles nested schemas, arrays, enums, and references

2. **Operation Processor** (`src/operation_processor.rs`)
   - Extracts API paths and operations
   - Processes parameters (query, path, header, cookie)
   - Handles request bodies and responses

3. **Code Generators** (`src/main.rs`)
   - Language-specific generation functions
   - Template rendering with Tera
   - Context building with processed schemas and operations

### Type Mapping

| OpenAPI Type | TypeScript (Zod) | Python (Pydantic) | Golang |
|--------------|------------------|-------------------|--------|
| `string` | `z.string()` | `str` | `string` |
| `integer` | `z.number()` | `int` | `int` |
| `number` | `z.number()` | `float` | `float64` |
| `boolean` | `z.boolean()` | `bool` | `bool` |
| `array` | `z.array(T)` | `List[T]` | `[]T` |
| `object` | `z.object({...})` | `BaseModel` | `struct` |
| `string(date)` | `z.date().or(z.string())` | `datetime` | `string` |

## Templates

Templates are located in the `templates/` directory:
- `templates/typescript/client.ts.tera` - TypeScript with Zod
- `templates/python/client.py.tera` - Python with Pydantic
- `templates/golang/client.go.tera` - Golang with native types

## Dependencies

- **Rust**: `openapiv3`, `tera`, `serde`, `clap`, `anyhow`, `indexmap`
- **TypeScript**: `zod` (required for generated code)
- **Python**: `pydantic`, `requests` (required for generated code)
- **Golang**: Standard library only

### Why `openapiv3`?

We evaluated multiple OpenAPI parsing libraries and chose `openapiv3` for the following reasons:

- **Simplicity**: Provides straightforward data structures perfect for code generation
- **Stability**: Mature, well-tested, and widely adopted (v2.0+)
- **Minimal dependencies**: Lightweight with only essential dependencies (serde, indexmap)
- **Sufficient for our use case**: Code generation doesn't require advanced validation or OpenAPI 3.1.x features

**Alternatives considered:**
- `oapi`: Better for validation/linting workflows with built-in `.check()` methods and schema composition operators, but adds unnecessary complexity for code generation
- `oas3`/`openapiv3_1`: Required for OpenAPI 3.1.x support, but we target 3.0.x specifications

## Limitations

- Currently supports OpenAPI 3.0 specifications
- Authentication schemes not yet implemented
- Error response handling is basic
- Multipart form data not supported

## Roadmap

- [ ] Authentication support (Bearer, API Key, OAuth2)
- [ ] Custom error types
- [ ] Multipart form data
- [ ] File upload/download
- [ ] Webhook support
- [ ] OpenAPI 3.1 support
- [ ] Additional languages (Java, C#, etc.)
- [ ] Full Rust client generation

## License

MIT
