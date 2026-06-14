# Rustify

Rustify compiles a deliberately strict subset of TypeScript into safe, readable Rust.
The current `0.1.0` MVP includes an Oxc-backed parser, shared analyzer, typed IR, Rust
code generator, CLI, language server, ESLint plugin scaffold, and VS Code extension
scaffold.

## Supported in 0.1

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

## Experimental hybrid mode

Hybrid mode attempts native Rust compilation first. If strict Rustify analysis
rejects otherwise valid TypeScript, it preserves the module graph as a fallback
bundle and records the decision in `rustify-hybrid.json`.

```json
{
  "entry": "src/main.ts",
  "out": "dist",
  "cargo": true,
  "package_name": "hybrid-app",
  "mode": "hybrid"
}
```

The current experimental fallback uses Node 22+ type transformation as an
external V8 host:

```bash
rustify compile
cd dist
npm run start
```

Native mode remains the default and continues rejecting unsupported dynamic
TypeScript.

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

Run the language server with:

```bash
cargo run -p rustify-lsp
```

Run the local playground with:

```bash
cargo run -p rustify-playground
```

Then open `http://127.0.0.1:3000`. The playground uses the same parser, analyzer,
diagnostics, typed IR, and Rust code generator as the CLI and LSP.

The language server provides diagnostics, Rust-target hovers, semantic tokens,
workspace/document symbols, quick fixes, go-to-definition, references, and
workspace rename across open documents and their relative imports. The VS Code
preview command requests generated Rust directly from the LSP, including unsaved
editor changes and isolated Rust modules. LSP ranges use UTF-16 positions, and
dynamic-type quick fixes are only offered when a concrete primitive type can be
inferred safely.

Use the ESLint plugin with ESLint 9 flat config:

```js
import rustify from "eslint-plugin-rustify";

export default [rustify.configs["flat/recommended"]];
```

## Architecture

```text
Oxc TypeScript parser -> normalized AST -> shared analyzer -> typed IR -> Rust codegen
                                              |
                                              +-> CLI / LSP / editor tooling
```

`PLAN.md` contains the broader roadmap. This repository implements the native
compiler/tooling milestones through basic async support and an experimental
external-V8 hybrid fallback, plus a shared-pipeline browser playground. Embedded
V8 and full TypeScript coverage remain later roadmap items.

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
