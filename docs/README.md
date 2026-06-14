# Rustify — Documentación Oficial

Bienvenido a la documentación oficial de **Rustify**, un compilador diseñado para transpilar un subconjunto estricto de TypeScript en código Rust seguro, rápido y legible.

---

## Índice de Componentes

A continuación se muestra el desglose de guías detalladas para cada una de las herramientas que componen el ecosistema de Rustify. Haz clic en el icono o en el enlace correspondiente para abrir la documentación:

| Herramienta | Icono | Descripción | Enlace |
| :--- | :---: | :--- | :--- |
| **CLI & Compilador** | <img src="../assets/icon-cli.png" width="48" height="48" /> | Compilador principal de terminal para chequear, explicar y transpilar código. | [Guía de CLI](./cli.md) |
| **Language Server (LSP)** | <img src="../assets/icon-lsp.png" width="48" height="48" /> | Servidor LSP para análisis inline, hovers de firmas Rust y navegación semántica. | [Guía de LSP](./lsp.md) |
| **Plugin de ESLint** | <img src="../assets/icon-eslint.png" width="48" height="48" /> | Linter estático para proyectos Node.js integrado con las reglas del compilador. | [Guía de ESLint](./eslint.md) |
| **Extensión de VS Code** | <img src="../assets/icon-vscode.png" width="48" height="48" /> | Extensión que gestiona el LSP y permite ver la previsualización de Rust en paralelo. | [Guía de VS Code](./vscode.md) |
| **Playground Web** | <img src="../assets/icon-playground.png" width="48" height="48" /> | Sandbox web interactivo para experimentar y previsualizar la compilación online. | [Guía de Playground](./playground.md) |
| **Bridge Híbrido** | <img src="../assets/logo.png" width="48" height="48" /> | Interoperabilidad síncrona nativo-JS por IPC y stdio para tipos complejos. | [Guía de Bridge Híbrido](./hybrid.md) |
| **Patrón Globals** | <img src="../assets/logo.png" width="48" height="48" /> | Guía y buenas prácticas para declarar variables y constantes globales de forma nativa. | [Guía de Globals](./globals.md) |

---

## Índice Temático General

1. [Guía de Inicio Rápido (Quickstart)](#cómo-iniciar-quickstart)
2. [Arquitectura Interna del Compilador](./architecture.md)
3. [Bridge de Interoperabilidad Híbrida](./hybrid.md)
4. [Gestión de Variables Globales (Patrón Globals)](./globals.md)

---

## Cómo Iniciar (Quickstart)

### Requisitos Previos

Asegúrate de tener instalados los siguientes componentes:

* [Rust & Cargo](https://rustup.rs/) (edición 2024 soportada)
* [Node.js](https://nodejs.org/) (versión >= 20 para herramientas de linter)

### 1. Clonar y Compilar el Proyecto

Puedes compilar el proyecto localmente utilizando el `Makefile` provisto en la raíz:

* **Modo Desarrollo (Debug):**
  ```bash
  make build-dev
  ```
* **Modo Producción (Release):**
  ```bash
  make build-release
  ```

El binario ejecutable compilado se encontrará en `target/debug/rustify` (para desarrollo) o `target/release/rustify` (para producción).

### Compilación y Uso con Podman / Docker

Si prefieres no instalar herramientas locales, puedes construir y ejecutar Rustify dentro de un contenedor:

1. **Construir la imagen:**
   ```bash
   make podman-build
   ```
2. **Ejecutar el compilador:**
   ```bash
   podman run --rm -v $(pwd):/workspace rustify:latest check app.ts
   ```

### 2. Tu Primer Archivo Rustify

Crea un archivo llamado `app.ts` con el siguiente contenido de TypeScript compatible con Rustify:

```ts
type User = {
  id: number;
  name: string;
};

export function greet(user: User): string {
  return `Hola, ${user.name}!`;
}
```

### 3. Verificar e Inspeccionar

Puedes comprobar si tu archivo cumple con las reglas estrictas de Rustify:

```bash
./target/release/rustify check app.ts
```

Para inspeccionar cómo el compilador bajará esto a Rust:

```bash
./target/release/rustify explain app.ts
```

### 4. Compilar a Rust

Para transpilar y compilar tu código a un archivo `.rs` ejecutable:

```bash
./target/release/rustify compile app.ts --out dist-rust
```

Esto generará el código Rust equivalente listo para usarse. Si deseas crear un proyecto Cargo completo con sus dependencias:

```bash
./target/release/rustify compile app.ts --out dist-rust --cargo
```
