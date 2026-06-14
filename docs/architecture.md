# Arquitectura del Compilador Rustify

Rustify está estructurado bajo un diseño modular compuesto por múltiples crates y paquetes interconectados que procesan y transforman el código fuente paso a paso.

## El Pipeline del Compilador

El flujo de compilación es el siguiente:

```text
TypeScript Source (.ts)
        │
        ▼
[rustify-parser]  ── back-end por Oxc parser
        │
        ▼
Normalized AST (Estructuras de datos Rustify)
        │
        ▼
[rustify-analyzer] ── Validador sintáctico, type-checker y tabla de símbolos
        │
        ▼
Typed IR (Representación Intermedia)
        │
        ▼
[rustify-codegen-rust] ── Generador de código a Rust legible y formateado (pretty)
        │
        ▼
Rust Source (.rs) / Cargo Project
```

---

## 1. Parser (`rustify-parser`)

Utiliza la biblioteca de parseo de alta velocidad de **Oxc** para procesar la sintaxis de TypeScript y transformarla en un AST interno normalizado en Rust. Esto desacopla el resto del compilador de las dependencias externas del parser de TypeScript.

## 2. Analizador Semántico y Checker (`rustify-analyzer`)

El analizador semántico es la parte central del compilador. Es responsable de:
- **Validación Sintáctica**: Rechazar construcciones dinámicas de JS incompatibles (`any`, `unknown`, `eval`, mutaciones de prototipos).
- **Type Checker**: Comprobación e inferencia de tipos básicos (`string`, `number`, `boolean`, `Result`, `Promise`).
- **Tabla de Símbolos**: Registro de structs, enums y funciones declaradas y exportadas.
- **Análisis de Mutabilidad**: Determinar si una variable `let` debe ser mut (`let mut`) en Rust en función de su uso posterior.

## 3. Representación Intermedia (`rustify-ir`)

Define las estructuras de datos fuertemente tipadas de nuestro AST interno. Aísla las fases de análisis semántico de la fase de generación de código, facilitando el análisis avanzado y hovers en el LSP.

## 4. Codegen (`rustify-codegen-rust`)

Toma el IR tipado validado y genera código Rust idiomático y seguro. Se asegura de mapear:
- `number` -> `f64`
- `string` -> `String`
- `T[]` -> `Vec<T>`
- Métodos como `push`/`pop`/`includes` a sus equivalentes seguros en Rust.
- Invocaciones a `console.log(...)` a macros de formateo `println!`.
