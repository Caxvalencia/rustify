# Herramientas de Desarrollo (LSP, ESLint, VSCode)

El ecosistema de Rustify incluye integraciones completas con herramientas de desarrollo para asegurar una experiencia fluida desde tu editor de código preferido.

## 1. Rustify LSP (`rustify-lsp`)

El Servidor de Lenguaje (Language Server Protocol) provee retroalimentación en tiempo real para archivos de TypeScript.

### Características:
- **Diagnósticos**: Muestra errores semánticos y advertencias al escribir.
- **Hovers**: Muestra el equivalente de Rust al posicionar el cursor sobre tipos y funciones de TypeScript (e.g. `number` -> `f64`).
- **Navegación**: Soporta Go to Definition, Find References y renombrado seguro de variables/tipos a lo largo de todo el espacio de trabajo.
- **Semantic Tokens**: Resaltado de sintaxis preciso basado en el tipo de símbolo (Struct, Enum, Function, Property).

---

## 2. Plugin ESLint (`eslint-plugin-rustify`)

Un linter complementario para detectar incompatibilidades de forma ultra rápida antes de que se ejecute la compilación.

### Instalación:
```bash
npm install eslint-plugin-rustify --save-dev
```

### Configuración (ESLint 9 Flat Config - `eslint.config.js`):
```js
import rustify from "eslint-plugin-rustify";

export default [
  rustify.configs["flat/recommended"]
];
```

El plugin utiliza internamente la CLI de `rustify check --json` para evaluar los diagnósticos de manera consistente con el compilador principal, eliminando código duplicado en JavaScript.

---

## 3. Extensión VSCode (`vscode-rustify`)

Provee integración nativa en VS Code.

### Características:
- **Cliente LSP**: Conecta VSCode automáticamente con `rustify-lsp`.
- **Vista Previa de Traducción (`rustify.preview`)**: Abre una ventana dividida al lado de tu editor actual que muestra el código Rust que se generaría a partir del código TypeScript activo (incluyendo cambios sin guardar).
- **Comando Check (`rustify.check`)**: Ejecuta un chequeo manual del archivo activo abriendo una terminal interna.
