# Rustify

Rustify compiles a deliberately strict subset of TypeScript into safe, readable Rust.
The current `0.1.0` MVP includes an Oxc-backed parser, shared analyzer, typed IR, Rust
code generator, CLI, language server, ESLint plugin scaffold, and VS Code extension
scaffold.

## Supported in 0.1

- Object type aliases and simple interfaces to Rust structs
- Simple enums
- Typed functions
- `string`, `number`, `boolean`, `void`, arrays, optional fields, and nullable unions
- Template literals and basic function bodies
- Shared diagnostics for unsupported dynamic TypeScript

Dynamic TypeScript features such as `any`, `unknown`, `eval`, decorators, namespaces,
prototype mutation, and non-nullable unions are rejected.

## Commands

```bash
cargo run -p rustify-cli -- check examples/greet.ts
cargo run -p rustify-cli -- explain examples/greet.ts
cargo run -p rustify-cli -- compile examples/greet.ts --out dist-rust
cargo run -p rustify-cli -- compile examples/greet.ts --out dist-rust --cargo
cargo run -p rustify-cli -- init my-rustify-project
```

Run the language server with:

```bash
cargo run -p rustify-lsp
```

## Architecture

```text
Oxc TypeScript parser -> normalized AST -> shared analyzer -> typed IR -> Rust codegen
                                              |
                                              +-> CLI / LSP / editor tooling
```

`PLAN.md` contains the broader roadmap. This repository implements its MVP 0.1
milestone; modules, async support, hybrid V8 fallback, and full TypeScript coverage
remain later roadmap items.

## Development

```bash
cargo fmt --all --check
cargo test --workspace
```

