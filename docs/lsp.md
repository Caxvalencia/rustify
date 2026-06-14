# Servidor de Lenguaje (LSP) — Rustify

<p align="center">
  <img src="../assets/icon-lsp.png" alt="Rustify LSP Icon" width="160" />
</p>

El módulo `rustify-lsp` implementa el protocolo **Language Server Protocol (LSP)** oficial para Rustify. Habilita una experiencia de desarrollo integrada y ágil directamente en tu editor de código favorito, proporcionando diagnósticos automáticos, autocompletado inteligente y hovers con la firma de Rust equivalente.

---

## Cómo Iniciar el LSP

El servidor del LSP viene compilado dentro del monorepo. Para ejecutarlo de forma aislada:

```bash
cargo run --package rustify-lsp
```

El proceso escuchará en la entrada/salida estándar (stdio) esperando conexiones de clientes LSP compatibles.

---

## Características Principales

### 1. Diagnósticos en Tiempo Real
El LSP compila e inspecciona tus archivos TypeScript al escribir. Si detecta violaciones a las reglas de Rustify, resalta la sintaxis inválida y muestra el error contextual:
* Uso de tipos dinámicos prohibidos (`any`, `unknown`).
* Omisión de anotaciones de tipos explícitas en parámetros o retornos de función.
* Mutaciones dinámicas no mapeables de JavaScript (`eval`, borrado de propiedades).

### 2. Equivalencia de Firmas (Hover)
Al colocar el cursor sobre un tipo, variable o función, el LSP te muestra su traducción a Rust en una ventana flotante.
* **Ejemplo**: Pasar el cursor sobre un parámetro con tipo `number` mostrará `Rust target: f64`.
* Pasar el cursor sobre `Promise<string>` mostrará `Rust target: impl Future<Output = String>`.

### 3. Navegación Inteligente
* **Go to Definition**: Salta directamente a la declaración de un tipo, struct, enum o función, inclusive si está importado desde otro módulo del workspace.
* **Find References**: Encuentra todas las ubicaciones donde se utiliza una variable o tipo en todo tu proyecto.

### 4. Renombrado Seguro (Rename Symbol)
Permite renombrar variables, funciones y estructuras en todo tu espacio de trabajo de forma segura y automatizada, actualizando todos los archivos de importación relativos.

---

## Configuración en Editores

El LSP es compatible con cualquier editor de texto moderno que admita clientes LSP.

### Visual Studio Code
La forma más sencilla de utilizar el LSP en VS Code es instalar la extensión oficial `vscode-rustify`, la cual inicia y gestiona el binario `rustify-lsp` automáticamente por ti.

### Neovim
Puedes configurar `rustify-lsp` en Neovim agregando la definición del binario en tu archivo de configuración de servidores de lenguaje utilizando `nvim-lspconfig`:

```lua
local lspconfig = require('lspconfig')
local configs = require('lspconfig.configs')

if not configs.rustify_lsp then
  configs.rustify_lsp = {
    default_config = {
      cmd = { 'rustify-lsp' },
      filetypes = { 'typescript' },
      root_dir = lspconfig.util.root_pattern('rustify.json', '.git'),
      settings = {},
    },
  }
end

lspconfig.rustify_lsp.setup{}
```
