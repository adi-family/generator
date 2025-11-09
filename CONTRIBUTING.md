# Contributing to Generator

Thank you for your interest in contributing to Generator! This document provides guidelines and instructions for contributing to the project.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Making Changes](#making-changes)
- [Testing](#testing)
- [Submitting Changes](#submitting-changes)
- [Coding Standards](#coding-standards)
- [Adding New Languages](#adding-new-languages)

## Code of Conduct

By participating in this project, you agree to maintain a respectful and inclusive environment for all contributors.

### Our Standards

- Use welcoming and inclusive language
- Be respectful of differing viewpoints and experiences
- Gracefully accept constructive criticism
- Focus on what is best for the community
- Show empathy towards other community members

## Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork** locally:
   ```bash
   git clone https://github.com/YOUR_USERNAME/generator.git
   cd generator
   ```
3. **Add upstream remote**:
   ```bash
   git remote add upstream https://github.com/adi-family/generator.git
   ```

## Development Setup

### Prerequisites

- Rust 1.70 or later
- Cargo (comes with Rust)
- Git

### Building the Project

```bash
# Build in debug mode
cargo build

# Build in release mode
cargo build --release

# Run the binary
./target/debug/generator --help
```

### Running Tests

```bash
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture

# Run a specific test
cargo test test_name
```

### Generating Example Code

```bash
# Generate TypeScript example
cargo run -- -s examples/petstore.yaml -l type-script -o generated/test-ts

# Generate Python example
cargo run -- -s examples/petstore.yaml -l python -o generated/test-py

# Generate Golang example
cargo run -- -s examples/petstore.yaml -l golang -o generated/test-go
```

## Project Structure

```
generator/
├── src/
│   ├── main.rs                 # CLI entry point and orchestration
│   ├── schema_processor.rs     # OpenAPI schema processing
│   ├── operation_processor.rs  # API operation extraction
│   └── config/
│       └── loader.rs           # Configuration loading
├── templates/
│   ├── typescript/             # TypeScript templates
│   ├── python/                 # Python templates
│   └── golang/                 # Golang templates
├── examples/
│   └── petstore.yaml           # Example OpenAPI specs
└── generated/                  # Output directory (gitignored)
```

### Key Components

- **`main.rs`**: CLI definition, argument parsing, and generator orchestration
- **`schema_processor.rs`**: Converts OpenAPI schemas to language-specific types
- **`operation_processor.rs`**: Extracts API operations and parameters
- **Templates**: Tera templates for each target language

## Making Changes

### Workflow

1. **Create a branch** from `main`:
   ```bash
   git checkout -b feature/your-feature-name
   ```

2. **Make your changes** with clear, focused commits

3. **Test your changes** thoroughly:
   ```bash
   cargo test
   cargo clippy
   cargo fmt
   ```

4. **Push to your fork**:
   ```bash
   git push origin feature/your-feature-name
   ```

5. **Open a Pull Request** on GitHub

### Commit Messages

Write clear, descriptive commit messages:

```
Add support for OAuth2 authentication

- Implement OAuth2 flow in schema processor
- Add OAuth2 configuration to generated clients
- Update templates for all languages
- Add tests for OAuth2 support

Fixes #123
```

**Format:**
- First line: Short summary (50 chars or less)
- Blank line
- Detailed description with bullet points
- Reference issues/PRs at the end

## Testing

### Writing Tests

Add tests for new functionality in the appropriate module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_conversion() {
        // Test implementation
        assert_eq!(expected, actual);
    }
}
```

### Manual Testing

After making changes, test with real OpenAPI specs:

1. Generate code for all supported languages
2. Verify the generated code compiles/runs
3. Check type safety and validation work correctly
4. Test with edge cases (optional fields, arrays, nested objects)

## Submitting Changes

### Pull Request Process

1. **Update documentation** if needed (README, inline comments)
2. **Add tests** for new functionality
3. **Ensure all tests pass**: `cargo test`
4. **Run the formatter**: `cargo fmt`
5. **Run the linter**: `cargo clippy`
6. **Update CHANGELOG.md** if applicable
7. **Create Pull Request** with a clear description

### Pull Request Template

```markdown
## Description
Brief description of the changes

## Type of Change
- [ ] Bug fix
- [ ] New feature
- [ ] Breaking change
- [ ] Documentation update

## Testing
How has this been tested?

## Checklist
- [ ] My code follows the project's style guidelines
- [ ] I have performed a self-review of my code
- [ ] I have commented my code where necessary
- [ ] I have updated the documentation
- [ ] My changes generate no new warnings
- [ ] I have added tests that prove my fix/feature works
- [ ] All tests pass locally
```

## Coding Standards

### Rust Style

Follow the official Rust style guidelines:

```bash
# Format code automatically
cargo fmt

# Check for common mistakes
cargo clippy

# Check for warnings
cargo check
```

### Best Practices

- **Use meaningful variable names**: `schema_name` not `sn`
- **Write documentation comments** for public APIs:
  ```rust
  /// Processes an OpenAPI schema and converts it to a target language type.
  ///
  /// # Arguments
  /// * `schema` - The OpenAPI schema reference
  /// * `language` - Target programming language
  ///
  /// # Returns
  /// The converted type as a string
  pub fn process_schema(schema: &Schema, language: &str) -> String {
      // Implementation
  }
  ```
- **Handle errors properly**: Use `Result<T, E>` and `anyhow` for error handling
- **Keep functions small and focused**: One responsibility per function
- **Add tests** for new functionality

### Template Style

- Use consistent indentation (2 or 4 spaces)
- Add comments explaining complex logic
- Keep templates readable and maintainable
- Test generated code compiles and runs

## Adding New Languages

To add support for a new programming language:

### 1. Create Template

Create `templates/<language>/client.<ext>.tera`:

```
templates/
└── java/
    └── client.java.tera
```

### 2. Add Type Mappings

In `src/schema_processor.rs`, add type conversion logic:

```rust
"java" => match openapi_type {
    "string" => "String",
    "integer" => "Integer",
    "number" => "Double",
    "boolean" => "Boolean",
    // ... more mappings
}
```

### 3. Update CLI

In `src/main.rs`, add the language to the enum:

```rust
#[derive(Debug, Clone, ValueEnum)]
enum Language {
    TypeScript,
    Python,
    Golang,
    Rust,
    Java,  // New language
}
```

### 4. Add Generation Logic

Add generation function in `src/main.rs`:

```rust
fn generate_java_code(
    spec: &OpenAPI,
    output_dir: &Path,
) -> Result<()> {
    // Implementation
}
```

### 5. Add Example

Create an example in the README showing generated code.

### 6. Document Dependencies

List required dependencies for the generated code.

### 7. Test Thoroughly

- Generate code from example specs
- Verify generated code compiles
- Test runtime validation works
- Add integration tests

## Questions?

- **Issues**: Open an issue on GitHub
- **Discussions**: Use GitHub Discussions for questions
- **Email**: Contact team@adi-family.com

## Recognition

Contributors will be acknowledged in:
- CHANGELOG.md for significant contributions
- GitHub contributors list
- Release notes

Thank you for contributing to Generator! 🎉