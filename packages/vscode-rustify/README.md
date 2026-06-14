# Rustify for VS Code

VS Code client for `rustify-lsp`.

Features:

- Live Rustify diagnostics and quick fixes
- Rust target hovers
- Go to definition, references, prepare rename, workspace/document symbols, and semantic tokens
- `Rustify: Check Current File`
- `Rustify: Preview Translation` opens generated Rust beside the active editor and
  includes unsaved document changes

Dynamic-type quick fixes are offered only when Rustify can safely infer the
replacement from an immutable primitive literal constant.

Install the `rustify` and `rustify-lsp` binaries on `PATH`, or configure
`rustify.cli.path` and `rustify.server.path`.
