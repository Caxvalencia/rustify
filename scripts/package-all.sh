#!/bin/bash
# No fallar en errores de cargo package para poder empaquetar el resto del monorepo
set -e

# Directorio de salida
DIST_DIR="$(pwd)/dist-packages"
rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"

echo "=== Empaquetando Crates de Cargo ==="
CRATES=(
  "crates/rustify-ir"
  "crates/rustify-parser"
  "crates/rustify-analyzer"
  "crates/rustify-codegen-rust"
  "crates/rustify-runtime"
  "crates/rustify-playground"
  "crates/rustify-lsp"
  "crates/rustify-cli"
)

for CRATE in "${CRATES[@]}"; do
  echo "Empaquetando $CRATE..."
  (
    cd "$CRATE"
    # cargo package puede dar error si las dependencias locales no estan publicadas en crates.io
    # Usamos || true para no abortar si falta publicacion en crates.io, pero reportamos
    if cargo package --allow-dirty --no-verify 2>/dev/null; then
      cp target/package/*.crate "$DIST_DIR/" 2>/dev/null || true
      echo "$CRATE empaquetado con éxito."
    else
      echo "Advertencia: $CRATE no se pudo empaquetar por dependencias locales no publicadas aún. Se empaquetará al publicar en crates.io en orden."
    fi
  )
done

echo "=== Empaquetando Modulos NPM ==="
# eslint-plugin-rustify
echo "Empaquetando packages/eslint-plugin-rustify..."
(
  cd packages/eslint-plugin-rustify
  npm pack --pack-destination "$DIST_DIR"
)

# vscode-rustify
echo "Empaquetando packages/vscode-rustify..."
(
  cd packages/vscode-rustify
  npx -y @vscode/vsce package --out "$DIST_DIR" --allow-missing-repository
)

echo "=== Contenido de dist-packages/ ==="
ls -lh "$DIST_DIR"
echo "=== Empaquetado de la suite completo! ==="
