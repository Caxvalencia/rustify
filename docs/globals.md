# Gestión de Variables y Constantes Globales (Patrón Globals) — Rustify

<p align="center">
  <img src="../assets/logo.png" alt="Rustify Logo" width="100" />
</p>

En el modo de compilación nativo de Rustify, declarar variables ejecutables o asignaciones libres en el ámbito global (fuera de las funciones) generará el error de diagnóstico `[SFT046]`. Esto se debe a que Rust exige condiciones estrictas de seguridad de hilos y ciclo de vida de memoria (`static` y `unsafe` mutabilidad) para variables globales.

Para resolver esta necesidad de forma limpia, estructurada y compatible con el estándar nativo de Rustify, implementamos el **Patrón Globals** utilizando la modularización nativa.

---

## El Patrón Globals

La alternativa recomendada consiste en encapsular todos los valores y configuraciones globales en un módulo TypeScript propio (por ejemplo, `globals.ts`) y exponerlos a través de funciones públicas de consulta. 

### Ventajas
1. **Seguridad Nativa**: Al transpilarse a Rust, las funciones se resuelven como llamadas limpias a funciones públicas de módulo en Rust, evitando el uso de variables estáticas inseguras (`unsafe static`).
2. **Modularidad**: Aísla todas las variables y configuraciones globales en un único archivo, facilitando el mantenimiento.
3. **Escalabilidad**: Permite en el futuro cambiar el origen de los datos (por ejemplo, leer variables de entorno en tiempo de ejecución o archivos JSON) simplemente modificando la función del módulo `globals.ts`, sin alterar los archivos que la importan.

---

## Ejemplo Práctico

### 1. Definición del Módulo de Configuración Global (`globals.ts`)

Crea un archivo TypeScript dedicado a tus constantes y configuraciones globales:

```typescript
// src/globals.ts

export function getAppName(): string {
  return "MiAplicacionRustify";
}

export function getTimeoutMs(): number {
  return 5000;
}

export function getMaxConnections(): number {
  return 20;
}
```

### 2. Consumo de las Variables Globales (`main.ts`)

Importa las funciones de consulta del módulo `globals.ts` en tu archivo de entrada o de lógica principal:

```typescript
// src/main.ts
import { getAppName, getTimeoutMs, getMaxConnections } from "./globals";

export function run(): void {
  console.log("Aplicación activa: " + getAppName());
  console.log("Timeout: " + getTimeoutMs() + "ms");
  console.log("Conexiones: " + getMaxConnections());
}
```

---

## Mapeo Generado en Rust

Rustify transpila este patrón a un sistema de módulos estructurado en Rust. Cada archivo TypeScript se mapea a su propio módulo `.rs` de forma aislada:

### globals.rs (Módulo de Configuración)
```rust
// dist-rust/globals.rs

pub fn get_app_name() -> String {
    "MiAplicacionRustify".to_string()
}

pub fn get_timeout_ms() -> f64 {
    5000.0
}

pub fn get_max_connections() -> f64 {
    20.0
}
```

### main.rs (Punto de entrada)
```rust
// dist-rust/main.rs
use crate::globals::{get_app_name, get_timeout_ms, get_max_connections};

pub fn run() {
    println!("Aplicación activa: {}", get_app_name());
    println!("Timeout: {}ms", get_timeout_ms());
    println!("Conexiones: {}", get_max_connections());
}
```
