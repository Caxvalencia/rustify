#!/bin/bash
set -e

echo "=== Verificando Sincronización de Versiones ==="

# 1. Obtener la versión de Cargo del Workspace (en el Cargo.toml raíz)
CARGO_VERSION=$(grep -m 1 "^version =" Cargo.toml | sed 's/version = //g' | tr -d '"' | tr -d ' ')
if [ -z "$CARGO_VERSION" ]; then
  # Probar con version.workspace o leer de workspace.package
  CARGO_VERSION=$(grep -A 5 "\[workspace.package\]" Cargo.toml | grep "^version =" | sed 's/version = //g' | tr -d '"' | tr -d ' ')
fi

echo "Versión del Workspace Cargo: $CARGO_VERSION"

# 2. Obtener versión de packages/eslint-plugin-rustify/package.json
ESLINT_VERSION=$(node -e "console.log(require('./packages/eslint-plugin-rustify/package.json').version)")
echo "Versión de eslint-plugin-rustify: $ESLINT_VERSION"

# 3. Obtener versión de packages/vscode-rustify/package.json
VSCODE_VERSION=$(node -e "console.log(require('./packages/vscode-rustify/package.json').version)")
echo "Versión de vscode-rustify: $VSCODE_VERSION"

# 4. Validar coincidencia
if [ "$CARGO_VERSION" != "$ESLINT_VERSION" ] || [ "$CARGO_VERSION" != "$VSCODE_VERSION" ]; then
  echo "Error: Las versiones del monorepo no están sincronizadas!"
  echo "Cargo Workspace: $CARGO_VERSION"
  echo "ESLint Plugin: $ESLINT_VERSION"
  echo "VSCode Extension: $VSCODE_VERSION"
  exit 1
fi

echo "Sincronización correcta! Todas las versiones coinciden en $CARGO_VERSION."
exit 0
