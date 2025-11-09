---
name: Bug Report
about: Report a bug or issue with the generator
title: '[BUG] '
labels: bug
assignees: ''
---

## Bug Description

A clear and concise description of the bug.

## Steps to Reproduce

1. Run the generator with: `generator --spec <path> --language <lang> --output <dir>`
2. Use the following OpenAPI spec: (provide spec or link)
3. Check the generated output
4. See error/issue

## Expected Behavior

What you expected to happen.

## Actual Behavior

What actually happened. Include error messages if applicable.

```
Error message here
```

## Environment

- **OS**: [e.g., macOS 13.0, Ubuntu 22.04, Windows 11]
- **Rust Version**: [e.g., 1.75.0]
- **Generator Version**: [e.g., 0.1.0]
- **Target Language**: [e.g., TypeScript, Python, Golang]

## OpenAPI Specification

If relevant, provide a minimal OpenAPI spec that reproduces the issue:

```yaml
# Paste your OpenAPI spec here or provide a link
openapi: 3.0.0
info:
  title: Example
  version: 1.0.0
paths:
  /example:
    get:
      responses:
        '200':
          description: Success
```

## Generated Code

If relevant, provide the problematic generated code:

```typescript
// Paste generated code here
```

## Additional Context

Add any other context about the problem here. Screenshots, links to related issues, etc.

## Possible Solution

If you have suggestions on how to fix the bug, please describe them here.