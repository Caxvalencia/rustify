# Rustify - Documentación Oficial

Bienvenido a la documentación oficial de **Rustify**, un compilador diseñado para transpilar un subconjunto estricto de TypeScript en código Rust seguro, rápido y legible.

## Índice de Documentación

1. [Guía de Inicio Rápido (Quickstart)](#cómo-iniciar-quickstart)
2. [Arquitectura del Compilador](./architecture.md)
3. [Guía de la Interfaz de Línea de Comandos (CLI)](./cli.md)
4. [Herramientas de Desarrollo (LSP, ESLint, VSCode)](./tooling.md)

---

## Cómo Iniciar (Quickstart)

### Requisitos Previos

Asegúrate de tener instalados los siguientes componentes:

- [Rust & Cargo](https://rustup.rs/) (edición 2024 soportada)
- [Node.js](https://nodejs.org/) (versión >= 20 para herramientas de linter)

### 1. Clonar y Compilar el Proyecto

Puedes compilar el proyecto localmente utilizando el `Makefile` provisto:

* **Modo Desarrollo (Debug):**
  ```bash
  make build:dev
  ```
* **Modo Producción (Release):**
  ```bash
  make build-release
  ```

El binario ejecutable compilado se encontrará en `target/debug/rustify-cli` (para desarrollo) o `target/release/rustify-cli` (para producción).

### Compilación y Uso con Podman

Si prefieres no instalar herramientas locales, puedes construir y ejecutar Rustify dentro de un contenedor usando Podman:

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
