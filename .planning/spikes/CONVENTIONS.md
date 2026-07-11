# Spike Conventions

Patterns and stack choices established across spike sessions. New spikes follow these unless the question requires otherwise.

## Stack

- **Rust** for core computation (WASM target)
- **Node.js** for format validation and reference implementations
- **wasm-pack** for WASM builds

## Structure

- Spikes numbered `NNN-descriptive-name`
- Each spike has a `README.md` with YAML frontmatter and Investigation Trail
- Source files included alongside when relevant

## Tools & Libraries

- `stl-io` v0.11.0 for STL parsing in Rust (zero dependencies)
- Node.js built-in `Buffer` for binary format reference
