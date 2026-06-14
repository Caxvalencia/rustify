# PLAN.md — Rustify Compiler

## Estado de ejecución

Leyenda:

* `[x]` Completo para el alcance definido.
* `[-]` Parcial: existe una implementación funcional, pero faltan partes del objetivo.
* `[ ]` Pendiente.

Resumen:

* `[x]` Compilador nativo MVP: parser Oxc, analyzer, IR tipado, codegen Rust y CLI.
* `[x]` Subconjunto v0.1: tipos, structs/interfaces simples, arrays, enums, funciones, variables, control de flujo y `console.log`.
* `[x]` Seguridad nativa: rechazo de sintaxis dinámica, comprobación de tipos, ownership básico y Rust generado compatible con `-D warnings`.
* `[x]` Runtime: JSON seguro, `Result`, async/await, `Promise<T>` y timers no bloqueantes.
* `[x]` Herramientas: LSP, extensión VSCode, plugin ESLint básico y playground.
* `[x]` Módulos: imports/exports relativos, aliases, re-exports, default exports, navegación, privacidad, namespaces Rust aislados y rechazo explícito de ciclos.
* `[x]` ESLint: las reglas básicas y autofixes seguros funcionan, y se consume directamente `rustify check --json` para eliminar la duplicación de reglas en JavaScript.
* `[-]` Modo híbrido: fallback funcional mediante Node 22+ como host V8 externo; falta V8 embebido, shims Rust y una estrategia de interoperabilidad nativo/JS.
* `[ ]` Rustify 1.0: estabilización de contratos, publicación, compatibilidad ampliada y validación multiplataforma de distribución.

Pendientes prioritarios:

1. `[x]` Representar módulos en IR/codegen sin aplanar todas las declaraciones en un único namespace Rust.
2. `[x]` Permitir helpers privados con el mismo nombre en módulos distintos sin colisiones.
3. `[x]` Eliminar la duplicación de reglas entre ESLint y `rustify-analyzer`, mediante bindings o un formato de diagnósticos compartido.
4. `[x]` Ampliar imports/exports: aliases, re-exports, default exports y rechazo explícito de ciclos.
5. `[ ]` Embebido V8 o runtime híbrido equivalente, con límites claros de interoperabilidad.
6. `[ ]` Endurecer distribución 1.0: paquetes publicables, pruebas reales de VSCode, benchmarks, fuzzing y matriz de compatibilidad.

## 1. Visión del proyecto

Crear un compilador/transpilador llamado Rustify que permita escribir un subconjunto estricto de TypeScript y compilarlo a Rust seguro, legible y mantenible.

Rustify debe funcionar primero como un lenguaje puente:

```txt
Rustify / TypeScript estricto
        ↓
Parser TS
        ↓
AST normalizado
        ↓
Análisis semántico
        ↓
IR tipado
        ↓
Generador Rust
        ↓
Cargo project / Rust source
```

La primera etapa no busca soportar todo TypeScript. Busca soportar un subconjunto fuertemente tipado que compile bien a Rust.

Además del compilador, Rustify debe proporcionar herramientas de desarrollo que permitan detectar incompatibilidades desde el editor antes de compilar.

En una segunda etapa, el proyecto podrá soportar TypeScript completo mediante una estrategia híbrida:

```txt
Código Rustify compatible
        → Compilación nativa a Rust

Código TypeScript dinámico/no soportado
        → Fallback a V8 / runtime JS embebido / shims Rust
```

---

## 2. Objetivo principal

Construir un compilador que reciba archivos `.ts` escritos bajo reglas Rustify y genere código Rust válido.

Como objetivo complementario, construir un ecosistema de herramientas de desarrollo compuesto por:

* Compilador Rustify.
* Rustify LSP.
* Plugin ESLint oficial.
* Extensión VSCode.
* Diagnósticos compartidos entre compilador y editor.

Ejemplo de entrada:

```ts
type User = {
  id: number
  name: string
  active: boolean
}

function greet(user: User): string {
  return `Hola ${user.name}`
}
```

Salida esperada:

```rust
#[derive(Clone, Debug)]
pub struct User {
    pub id: f64,
    pub name: String,
    pub active: bool,
}

pub fn greet(user: User) -> String {
    format!("Hola {}", user.name)
}
```

---

## 3. Principios de diseño

### 1. Primero seguridad, luego cobertura

* No intentar soportar todo TypeScript desde el inicio.
* Rechazar código ambiguo, dinámico o imposible de traducir limpiamente.

### 2. Rust idiomático, pero generado

* El Rust generado debe ser razonablemente legible.
* Priorizar `String`, `Vec<T>`, `Option<T>`, `Result<T, E>`, `HashMap<K, V>`.
* Evitar lifetimes complejos en el MVP.

### 3. Errores claros

* Cuando algo no sea compilable, el compilador debe explicar por qué.
* Mensajes tipo:

```txt
Rustify error: `any` is not allowed in native Rust mode.
Hint: use a concrete type or enable hybrid runtime mode.
```

### 4. Linter de desarrollo (Rustify LSP / ESLint Plugin)

Rustify debe incluir desde etapas tempranas un sistema de análisis estático para desarrolladores.

Objetivos:

* Detectar errores antes de ejecutar el compilador.
* Compartir reglas con el compilador.
* Reducir ciclos de feedback.
* Facilitar la adopción de Rustify.
* Guiar al desarrollador hacia código compatible con Rust.

Componentes:

```txt
Rustify Compiler
        ↑
        │ reglas compartidas
        ↓
Rustify Analyzer Core
        ↑
        ├── Rustify CLI
        ├── Rustify LSP
        ├── ESLint Plugin
        └── VSCode Extension
```

Capacidades:

* Diagnósticos en tiempo real.
* Quick Fixes.
* Hover informativo.
* Code Actions.
* Navegación de símbolos.
* Explicación de incompatibilidades.
* Compatibilidad con CI.

Validaciones iniciales:

* Uso de `any`.
* Uso de `unknown`.
* Falta de tipos explícitos.
* Unions no soportadas.
* Uso de `eval`.
* Objetos dinámicos.
* Mutaciones de prototipos.
* Decorators.
* APIs incompatibles con Rust.
* Imports no soportados.
* Uso de `this` dinámico.

Ejemplo:

```txt
Rustify Linter [SFT001]

`any` is not supported by Rustify.

Suggestion:
Replace `any` with a concrete type.
```

Quick Fix sugerido:

```ts
const value: any = "hello"
```

↓

```ts
const value: string = "hello"
```

El objetivo es que la mayoría de errores sean detectados mientras el usuario escribe código.

### 5. Arquitectura por fases

* Parser y AST.
* Validador Rustify.
* Analizador semántico.
* IR tipado.
* Codegen Rust.
* Runtime/shims.
* Linter/LSP.
* Fallback V8 experimental.

### 6. Modo híbrido como extensión

* El MVP debe compilar sin V8.
* V8 debe aparecer después como fallback para TypeScript dinámico.

---

## 4. Nombre interno del proyecto

Nombre de paquete sugerido:

```txt
rustify-rs
```

CLI sugerida:

```bash
rustify compile src/main.ts --out dist-rust
rustify check src/main.ts
rustify explain src/main.ts
rustify init
```

Herramientas adicionales:

```bash
rustify-lsp
eslint --plugin rustify
```

---

## 5. Stack técnico sugerido

### Lenguaje principal

```txt
Rust
```

### Parser TypeScript

Evaluar y elegir uno:

```txt
Opción A: SWC
Opción B: Oxc
```

### Núcleo compartido de análisis

Crear un crate reutilizable:

```txt
rustify-analyzer
```

Este crate será utilizado por:

```txt
rustify-cli
rustify-lsp
eslint-plugin-rustify
future vscode extension
```

### LSP

Opciones:

```txt
tower-lsp
lsp-types
```

### ESLint Plugin

Opciones:

```txt
eslint
typescript-eslint
node bindings
wasm bindings opcionales
```

### CLI

```txt
clap
```

### Diagnósticos

```txt
ariadne
miette
codespan-reporting
```

### Generación de código Rust

```txt
quote
proc-macro2
prettyplease
```

### Testing

```txt
cargo test
insta snapshots
trybuild opcional
```

### Formateo del Rust generado

```bash
rustfmt
```

---

## 6. Estructura inicial del repositorio

```txt
rustify-rs/
  Cargo.toml
  README.md
  PLAN.md

  crates/
    rustify-cli/
      Cargo.toml
      src/
        main.rs

    rustify-parser/
      Cargo.toml
      src/
        lib.rs
        parser.rs
        ast_adapter.rs

    rustify-analyzer/
      Cargo.toml
      src/
        lib.rs
        rustify_rules.rs
        diagnostics.rs
        type_checker.rs
        symbol_table.rs

    rustify-lsp/
      Cargo.toml
      src/
        main.rs
        server.rs
        diagnostics.rs

    rustify-ir/
      Cargo.toml
      src/
        lib.rs
        ir.rs
        types.rs

    rustify-codegen-rust/
      Cargo.toml
      src/
        lib.rs
        emit.rs
        emit_expr.rs
        emit_stmt.rs
        emit_type.rs

    rustify-runtime/
      Cargo.toml
      src/
        lib.rs

  packages/
    eslint-plugin-rustify/
      package.json
      src/
        index.ts
        rules/

    vscode-rustify/
      package.json
      src/

  examples/
  tests/
```

---

## 7. Rustify v0.1: subconjunto permitido

### 7.1 Tipos primitivos soportados

TypeScript:

```ts
string
number
boolean
void
null
undefined
```

Rust:

```rust
String
f64
bool
()
Option<T>
Option<T>
```

Regla inicial:

```txt
number → f64
```

Más adelante se puede soportar:

```ts
int
float
u32
i32
f64
```

mediante tipos branded o aliases especiales.

---

### 7.2 Tipos estructurados

Soportar:

```ts
type User = {
  id: number
  name: string
}
```

Generar:

```rust
#[derive(Clone, Debug)]
pub struct User {
    pub id: f64,
    pub name: String,
}
```

También soportar interfaces simples.

El linter debe validar estas estructuras en tiempo real y sugerir correcciones cuando detecte patrones incompatibles.

---

### 7.3 Arrays

Mapeo:

```txt
T[] → Vec<T>
Array<T> → Vec<T>
```

El LSP debe mostrar información contextual:

```txt
Rust target:
Vec<T>
```

al posicionarse sobre tipos compatibles.

---

### 7.4 Objetos opcionales

Regla:

```txt
campo?: T → Option<T>
```

El editor debe mostrar sugerencias automáticas indicando que el campo será convertido a `Option<T>`.

---

### 7.5 Union types limitados

Soportar únicamente:

```ts
T | null
T | undefined
```

El linter debe marcar inmediatamente:

```ts
string | number
```

como error de compatibilidad.

---

### 7.6 Enums

Soportar enums simples.

El LSP debe mostrar la representación Rust equivalente mediante hover.

---

### 7.7 Funciones

Reglas:

* Todos los parámetros deben tener tipo explícito.
* Toda función debe tener tipo de retorno explícito.
* No permitir `any`.

El ESLint Plugin debe ofrecer autofixes cuando sea posible.

---

### 7.8 Variables

Reglas:

```txt
const → let
let → let mut
```

El editor debe poder mostrar la traducción aproximada a Rust.

---

### 7.9 Condicionales

Soportadas.

El linter debe validar expresiones booleanas compatibles.

---

### 7.10 Loops

Soportar:

```ts
for...of
while
```

El linter debe rechazar patrones no soportados.

---

### 7.11 Console

Mapeo:

```txt
console.log → println!
```

El hover del LSP debe mostrar esta equivalencia.

---

## 8. Rustify v0.1: cosas prohibidas

El compilador y el linter deben rechazar:

```ts
any
unknown
eval
with
prototype mutation
dynamic property assignment
delete obj.prop
this dinámico
Function constructor
monkey patching
decorators
reflect metadata
index signatures dinámicas
implicit any
namespace
ambient declarations
global augmentation
```

Todos estos errores deben aparecer:

* En CLI.
* En LSP.
* En ESLint.
* En CI.

---

## 9. IR intermedio

Crear un IR propio para no acoplar el compilador directamente al AST de SWC/Oxc.

Además, el LSP debe poder consumir parcialmente este IR para:

* Mostrar información de tipos.
* Navegar símbolos.
* Explicar conversiones Rust.

---

## 10. Pipeline del compilador

Implementar este flujo:

```txt
1. Leer archivo .ts
2. Parsear a AST TypeScript
3. Convertir AST externo a AST interno normalizado
4. Validar reglas Rustify
5. Construir tabla de símbolos
6. Resolver tipos
7. Generar IR tipado
8. Emitir Rust
9. Ejecutar rustfmt
10. Opcional: generar Cargo project
```

Pipeline extendido para herramientas:

```txt
Parser
  ↓
Analyzer Core
  ↓
Compiler
LSP
ESLint Plugin
VSCode Extension
```

---

## 11. Fase 1 — Setup del repositorio `[x]`

Tareas:

* Crear workspace Cargo.
* Crear crates principales.
* Crear CLI mínima.
* Agregar parser seleccionado.
* Crear crate `rustify-analyzer`.
* Preparar arquitectura para LSP.

Criterios de aceptación:

* `cargo test` funciona.
* Workspace compilable.
* Analyzer reutilizable.

---

## 12. Fase 2 — Validador Rustify `[x]`

Implementar reglas de rechazo.

Además:

* Exponer diagnósticos reutilizables.
* Compartir códigos de error entre CLI y LSP.
* Crear formato estándar de errores.

---

## 13. Fase 3 — Type checker mínimo `[x]`

Implementar:

* Tabla de símbolos global.
* Tabla de símbolos por función.
* Registro de structs.
* Registro de enums.
* Resolución de identificadores.
* Validación de asignaciones.
* Validación de return.
* Validación de llamadas a funciones.
* Validación de acceso a propiedades.

El LSP debe reutilizar exactamente este sistema.

---

## 14. Fase 4 — Linter de desarrollo (Rustify LSP / ESLint Plugin) `[-]`

Objetivo:

Construir la primera experiencia de desarrollo integrada.

### Rustify LSP

Comando:

```bash
rustify-lsp
```

Capacidades:

* Diagnósticos en tiempo real.
* Hover.
* Go to Definition.
* Find References.
* Rename Symbol.
* Code Actions.
* Quick Fixes.
* Semantic Tokens.
* Document Symbols.

### ESLint Plugin

Instalación:

```bash
npm install eslint-plugin-rustify
```

Configuración:

```json
{
  "plugins": ["rustify"],
  "extends": ["plugin:rustify/recommended"]
}
```

Reglas iniciales:

```txt
rustify/no-any
rustify/no-unknown
rustify/no-eval
rustify/no-dynamic-object
rustify/no-unsupported-union
rustify/explicit-return-type
rustify/explicit-param-types
```

### VSCode Extension

Funciones:

* Integración con Rustify LSP.
* Diagnósticos inline.
* Hover Rust equivalente.
* Vista previa de traducción.
* Quick Fixes.

### Criterios de aceptación

* `[x]` Diagnósticos visibles en VSCode.
* `[x]` ESLint detecta incompatibilidades.
* `[x]` Reutilización del Analyzer Core: CLI, LSP, playground y ESLint lo reutilizan directamente.
* `[x]` Sin duplicación de reglas entre analyzer y ESLint.

---

## 15. Fase 5 — Generación de IR `[x]`

Convertir código válido a IR.

El LSP podrá utilizar este IR para mostrar información avanzada.

---

## 16. Fase 6 — Codegen Rust v0.1 `[x]`

Generar Rust para:

* Structs.
* Enums.
* Funciones.
* Variables.
* Condicionales.
* Loops.
* Operaciones básicas.

Además:

```bash
rustify explain file.ts
```

debe mostrar cómo se traducirá cada elemento.

---

## 17. Fase 7 — Runtime Rustify `[x]`

Crear crate:

```txt
rustify-runtime
```

Objetivo:

* Centralizar helpers.
* Compartir utilidades.
* Facilitar futuras integraciones híbridas.

---

## 18. Fase 8 — Soporte para módulos `[x]`

Soportar:

```ts
export
import
```

El LSP debe resolver símbolos entre módulos.

Estado:

* `[x]` Imports relativos con nombres.
* `[x]` Exports explícitos y rechazo de imports privados.
* `[x]` Resolución, navegación y análisis básico entre módulos en CLI/LSP.
* `[x]` Validación de alcance para impedir el uso implícito de símbolos privados.
* `[x]` Namespaces reales en IR/codegen, con imports Rust explícitos y helpers privados aislados.
* `[x]` Nombres de archivo normalizados a identificadores de módulo Rust seguros.
* `[x]` Aliases de imports para tipos y funciones, incluyendo navegación LSP.
* `[x]` Re-exports nombrados transitivos, incluyendo aliases y navegación LSP al origen.
* `[x]` Política de ciclos explícita: los grafos cíclicos se rechazan antes del análisis/codegen.
* `[x]` Default exports/imports nombrados para tipos y funciones, incluyendo navegación LSP.

---

## 19. Fase 9 — Testing `[x]`

### Tests del compilador

* Parser.
* Analyzer.
* IR.
* Codegen.

### Tests del LSP

* Hover.
* Diagnostics.
* Rename.
* References.

### Tests ESLint

* Reglas válidas.
* Reglas inválidas.
* Autofixes.

### Integración

```txt
TypeScript
      ↓
Rustify Analyzer
      ↓
CLI / LSP / ESLint
```

Estado:

* `[x]` Tests unitarios y de integración para parser, analyzer, codegen, CLI, LSP y runtime.
* `[x]` Tests del plugin ESLint y validación sintáctica de la extensión VSCode.
* `[x]` Gate CI que compila ejemplos como proyectos Cargo aislados y ejecuta fallback/playground.
* `[x]` Tests end-to-end reales dentro de VSCode y pruebas de distribución.

---

## 20. Roadmap resumido

### MVP 0.1 `[x]`

* CLI.
* Parser.
* Analyzer.
* Linter básico.
* LSP básico.
* ESLint Plugin básico.
* Structs.
* Funciones.
* Arrays.
* Enums.
* Rust codegen.

### MVP 0.2 `[x]`

* VSCode Extension.
* Quick Fixes.
* Hover avanzado.
* Módulos.
* Mejor inferencia.

### MVP 0.3 `[x]`

* Compatibilidad ampliada.
* JSON.
* Result.
* Mejor experiencia de desarrollo.

### MVP 0.4 `[x]`

* Async/await.
* Promise.
* Runtime async.

### MVP 0.5 `[-]`

* Modo híbrido.
* Integración V8.
* Explicaciones avanzadas.

Implementado con fallback V8 externo mediante Node 22+; V8 embebido e interoperabilidad híbrida siguen pendientes.

### 1.0 `[ ]`

* Rustify estable.
* LSP completo.
* ESLint Plugin estable.
* VSCode Extension oficial.
* Playground.
* LSP multiplataforma.
* Compatibilidad incremental con TypeScript real.

---

## 21. Filosofía final

Rustify no debe ser simplemente un compilador.

Debe convertirse en una plataforma completa de desarrollo:

```txt
TypeScript estricto
        ↓
Rustify Analyzer
        ↓
Linter + LSP + Compiler
        ↓
Rust seguro y mantenible
```

La meta es que el desarrollador reciba feedback inmediato desde el editor, pueda corregir incompatibilidades antes de compilar y obtenga como resultado código Rust seguro, rápido y predecible.
