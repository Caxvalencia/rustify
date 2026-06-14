# Gestión de Variables y Constantes Globales — Rustify

<p align="center">
  <img src="../assets/logo.png" alt="Rustify Logo" width="100" />
</p>

En el modo de compilación nativo de Rustify, la declaración de variables y constantes globales se maneja de forma eficiente y segura. Para ello, dispones de dos enfoques: el uso de **constantes globales estáticas directas** y el **patrón de funciones globales**.

---

## 1. Constantes Globales Directas (`const`) — *Recomendado*

El compilador de Rustify admite la definición directa de constantes globales utilizando la palabra clave `const` a nivel de módulo. El compilador detecta automáticamente estos valores y los transcompila a constantes nativas en Rust en formato `SCREAMING_SNAKE_CASE` de manera automática, sin generar el diagnóstico `SFT046`.

### Tipos Soportados y Restricciones de Rust
Debido a que Rust requiere que los valores de las constantes se resuelvan en tiempo de compilación y no realicen asignaciones en la memoria dinámica (heap), los tipos admitidos están limitados a:
- **String**: Se mapean automáticamente a referencias estáticas `&'static str` en Rust.
- **Number**: Se mapean a `f64`.
- **Boolean**: Se mapean a `bool`.

*Nota: Los arrays (vectores) y objetos JSON requieren asignación en el heap, por lo que no pueden declararse como constantes directas en el ámbito global.*

### Ejemplo Práctico

#### `globals.ts` (Definición de constantes)
```typescript
// Define y exporta tus constantes en camelCase o snake_case
export const appName = "MiAplicacionRustify";
export const timeoutMs = 5000;
export const maxConnections = 20;
```

#### `main.ts` (Consumo directo de las constantes)
```typescript
import { appName, timeoutMs, maxConnections } from "./globals";

export function run(): void {
  console.log("Aplicación activa: " + appName);
  console.log(`Timeout: ${timeoutMs}ms`);
  console.log(`Conexiones: ${maxConnections}`);
}
```

### Mapeo Generado en Rust

El compilador traduce de forma transparente las variables y sus referencias a la nomenclatura recomendada en Rust:

#### `globals.rs`
```rust
// Generado automáticamente por Rustify
pub const APP_NAME: &'static str = "MiAplicacionRustify";
pub const TIMEOUT_MS: f64 = 5000.0;
pub const MAX_CONNECTIONS: f64 = 20.0;
```

#### `main.rs`
```rust
use crate::globals::{APP_NAME, TIMEOUT_MS, MAX_CONNECTIONS};

pub fn run() {
    println!("{:?}", format!("{}{}", "Aplicación activa: ".to_string(), APP_NAME));
    println!("{}", format!("Timeout: {}ms", TIMEOUT_MS));
    println!("{}", format!("Conexiones: {}", MAX_CONNECTIONS));
}
```

---

## 2. El Patrón Globals (Envoltura en Funciones)

Si necesitas utilizar estructuras complejas (como arrays u objetos JSON) o valores globales que dependan de lógica en tiempo de ejecución (como leer variables de entorno o configuraciones dinámicas), la alternativa recomendada es encapsular el acceso a través de funciones públicas de consulta. 

### Ejemplo de Configuración Dinámica/Compleja

#### `globals.ts`
```typescript
export function getAppName(): string {
  return "MiAplicacionRustify";
}

// Útil para inicializar objetos o datos complejos
export function getLimitesConexiones(): number[] {
  return [10, 20, 50];
}
```

#### `main.ts`
```typescript
import { getAppName, getLimitesConexiones } from "./globals";

export function run(): void {
  console.log("Aplicación activa: " + getAppName());
  const limites = getLimitesConexiones();
  console.log(`Límite máximo: ${limites[1]}`);
}
```

### Mapeo Generado en Rust

#### `globals.rs`
```rust
pub fn get_app_name() -> String {
    "MiAplicacionRustify".to_string()
}

pub fn get_limites_conexiones() -> Vec<f64> {
    vec![10.0, 20.0, 50.0]
}
```

#### `main.rs`
```rust
use crate::globals::{get_app_name, get_limites_conexiones};

pub fn run() {
    println!("{:?}", format!("{}{}", "Aplicación activa: ".to_string(), get_app_name()));
    let limites = get_limites_conexiones();
    println!("{}", format!("Límite máximo: {}", limites[1]));
}
```
