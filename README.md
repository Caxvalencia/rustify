# Rustify

<p align="center">
  <img src="assets/logo.png" alt="Rustify Logo" width="160" />
</p>

Rustify compiles a deliberately strict subset of TypeScript into safe, readable Rust.
The current `1.0.0` release includes an Oxc-backed parser, shared analyzer, typed IR, Rust
code generator, CLI, language server, ESLint plugin, and VS Code extension.

## Supported in 1.0

- Object type aliases and simple interfaces to Rust structs
- Typed object literals, nested structs, and omitted optional fields
- Reusable non-`Copy` field access through explicit Rust clones
- Reusable non-`Copy` bindings across calls and assignments
- Typed empty and populated array literals
- Nested struct property access and `array.length`
- Safe `array[index]` reads, native `array.includes`/`join`, mutable-local
  `array.push`/`pop`, and string methods
- Simple enums
- Typed functions
- Explicit empty `return;` in `void` functions
- `string`, `number`, `boolean`, `void`, arrays, optional fields, and nullable unions
- Variables, assignments, function calls, property access, and template literals
- Checked binding mutability: `const` and parameters cannot be reassigned; `let` can
- Arithmetic, remainder, comparisons, string concatenation, and boolean logic
- Typed conditional (`condition ? value : fallback`) expressions
- Native numeric `Math` helpers: `abs`, `floor`, `ceil`, `round`, `min`, `max`, and `pow`
- Typed `if`/`else`, `while`, and `for...of` control flow with `break` and `continue`
- Exhaustive return validation for every non-`void` function path
- Relative named and aliased imports, transitive named re-exports, and exported
  declarations across isolated `.ts` modules
- Named default function/type exports and default imports
- Rust module emission with explicit imports, private declaration isolation, and safe
  generated module identifiers
- Explicit rejection of cyclic native module graphs
- Enum variants, nullable values (`Some`/`None`), and `void` functions
- Safe `Result<T, E>` values and JSON parsing/stringifying through `rustify-runtime`
- Native `async` functions, `await`, and `Promise<T>` lowering to Rust futures
- Typed multi-argument `console.log(...values)` lowering to one Rust `println!`
- Idiomatic Rust identifier generation from TypeScript camelCase
- UpperCamelCase Rust generation for type and enum variant identifiers
- Early diagnostics for names that collide after Rust identifier normalization
- Shared diagnostics for unsupported dynamic TypeScript

Dynamic TypeScript features such as `any`, `unknown`, `eval`, decorators, namespaces,
prototype mutation, and non-nullable unions are rejected.

`array.push(value)` is supported only as a standalone statement on a local array
declared with `let`; using its JavaScript length return value is rejected.
`array.pop()` returns `T | null`, represented as Rust `Option<T>`.
`array[index]` also returns `T | null`; negative, fractional, and out-of-bounds
indices produce `null` instead of panicking.
Nullable values support safe `isSome()`, `isNone()`, and `unwrapOr(fallback)`
operations. Rustify deliberately does not expose a panicking `unwrap()`.

Safe JSON APIs return `Result` instead of throwing:

```ts
function parseDocument(input: string): Result<JsonValue, string> {
  return JSON.parse(input)
}
```

Result values support safe `isOk()`, `isErr()`, and `unwrapOr(fallback)`
operations. Panicking `unwrap()` and `unwrapErr()` operations are intentionally
unsupported.

Cargo projects that use JSON automatically vendor `rustify-runtime`.

Async Rustify functions declare `Promise<T>` and compile to native Rust `async fn`:

```ts
async function loadMessage(): Promise<string> {
  return "ready"
}

async function relayMessage(): Promise<string> {
  return await loadMessage()
}
```

`Promise<T>` is supported as a direct async return or parameter value. Promise
fields, nested Promise containers, and stored Promise bindings are rejected
because Rust cannot represent them with the generated `impl Future` types.
Promises used as standalone statements must be awaited because Rust futures are
lazy and otherwise would never execute. Ignored `Option` and `Result` values are
discarded explicitly in generated Rust to keep intentional TypeScript
expression statements warning-free.
Direct Promise parameters must be consumed exactly once because generated Rust
futures are move-only. Ordinary unused parameters are explicitly acknowledged
by generated Rust so valid Rustify functions remain warning-free.

Local `let` bindings only become Rust `mut` bindings when later code actually
mutates them. Unused locals and `for...of` bindings are also acknowledged
explicitly, keeping generated projects compatible with `-D warnings`. Usage and
mutation analysis respects lexical shadowing between parameters, local
variables, branches, and loop bindings. Statements after terminal control flow
are omitted from generated Rust so unreachable TypeScript does not introduce
Rust warnings or affect mutability analysis.

The async runtime provides non-blocking timers:

```ts
async function pause(milliseconds: number): Promise<void> {
  await Rustify.sleep(milliseconds)
}
```

## Hybrid mode

Hybrid mode (`--mode hybrid`) enables native Rust compilation combined with dynamic Node.js fallback execution at the function level:
1. **Function-Level Annotation**: Mark specific functions with a `/** @hybrid */` JSDoc comment.
2. **Type Checking Bypass**: Inside hybrid functions, dynamic types like `any` are allowed and will not stop native compilation.
3. **IPC Fallback Lowering**: The body of a hybrid function is replaced in the generated Rust with a synchronous IPC/JSON call (`rustify_runtime::call_js_fallback`).
4. **Source Copying**: Original TypeScript sources are automatically copied to the `fallback/` directory in your compilation output to be loaded dynamically by Node.js using `--experimental-transform-types` at runtime.

```json
{
  "entry": "src/main.ts",
  "out": "dist",
  "cargo": true,
  "package_name": "hybrid-app",
  "mode": "hybrid"
}
```

Native mode remains the default and continues rejecting unsupported dynamic TypeScript unless explicitly annotated with `/** @hybrid */`.

## Commands

```bash
cargo run -p rustify-cli -- check examples/greet.ts
cargo run -p rustify-cli -- explain examples/greet.ts
cargo run -p rustify-cli -- explain examples/greet.ts --json
cargo run -p rustify-cli -- compile examples/greet.ts --out dist-rust
cargo run -p rustify-cli -- compile examples/greet.ts --out dist-rust --cargo
cargo run -p rustify-cli -- init my-rustify-project
```

Inside an initialized project, commands resolve `rustify.json` automatically:

```bash
rustify check
rustify compile
rustify compile --no-cargo
rustify explain
rustify --config path/to/rustify.json compile --out custom-output
```

`rustify explain` prints the inferred Rust signature, statement-level lowering
decisions, safe collection/runtime mappings, and the generated Rust source. Use
`--json` to inspect the complete typed IR.

Project configuration:

```json
{
  "entry": "src/main.ts",
  "out": "dist-rust",
  "cargo": true,
  "package_name": "rustify-output"
}
```

## Ecosystem & Tools

Rustify features a complete developer ecosystem consisting of multiple specialized tools. Explore the detailed documentation for each component below:

| Tool | Icon | Description | Guide |
| :--- | :---: | :--- | :--- |
| **CLI & Compiler** | <img src="assets/icon-cli.png" width="40" height="40" /> | Transpile TypeScript into safe, warning-free Rust. | [CLI Guide](docs/cli.md) |
| **Language Server (LSP)** | <img src="assets/icon-lsp.png" width="40" height="40" /> | Real-time diagnostics, Rust-equivalent hovers, and semantic navigation. | [LSP Guide](docs/lsp.md) |
| **ESLint Plugin** | <img src="assets/icon-eslint.png" width="40" height="40" /> | Static compatibility checks built directly into Node.js tools. | [ESLint Guide](docs/eslint.md) |
| **VS Code Extension** | <img src="assets/icon-vscode.png" width="40" height="40" /> | Native editor integration with real-time translation preview panels. | [VS Code Guide](docs/vscode.md) |
| **Interactive Playground** | <img src="assets/icon-playground.png" width="40" height="40" /> | Web-based editor sandbox to experiment with compilation directly. | [Playground Guide](docs/playground.md) |
| **Globals Pattern** | <img src="assets/logo.png" width="40" height="40" /> | Design pattern for managing global constants and variables natively. | [Globals Guide](docs/globals.md) |

For further details on how the compiler works under the hood and how to leverage the hybrid IPC bridge, please read the [Architecture Guide](docs/architecture.md), [Hybrid Interoperability Bridge Guide](docs/hybrid.md), and [Globals Guide](docs/globals.md).


## Architecture

```text
Oxc TypeScript parser -> normalized AST -> shared analyzer -> typed IR -> Rust codegen
                                              |
                                              +-> CLI / LSP / editor tooling
```

This repository implements the native compiler/tooling milestones through basic async support, a synchronous hybrid fallback bridge, and a shared-pipeline browser playground. Embedded V8 and full TypeScript coverage remain later roadmap items.

## Development

```bash
cargo fmt --all --check
cargo test --workspace
./scripts/ci.sh
```

GitHub Actions runs the Rust workspace and editor packages on Linux, macOS, and
Windows. The Linux integration gate additionally compiles every supported example
as an isolated Cargo project, verifies the intentionally invalid example, and
executes the external-V8 hybrid fallback and playground API.
