# Interfaz de Línea de Comandos (CLI) — Rustify

<p align="center">
  <img src="../assets/icon-cli.png" alt="Rustify CLI Icon" width="160" />
</p>

El módulo `rustify-cli` provee la herramienta principal por línea de comandos para compilar, verificar e interactuar con el compilador de Rustify. Procesa el código fuente TypeScript estricto y genera código Rust ejecutable de forma instantánea.

---

## Instalación y Construcción

Para compilar el binario de la CLI localmente, asegúrate de tener instalado Rust (edición 2024). Ejecuta el siguiente comando en el directorio raíz del proyecto:

```bash
make build-release
```

El ejecutable compilado estará disponible en:
```bash
./target/release/rustify
```

O si utilizas Cargo directamente:
```bash
cargo run --package rustify-cli -- [opciones] [comando]
```

---

## Comandos Disponibles

### 1. `check`
Verifica si uno o más archivos de TypeScript cumplen con el subconjunto estricto de Rustify.

```bash
rustify check src/main.ts
```

* **Salida Estándar**: Si el código es válido, no reporta errores. Si infringe las reglas (como el uso de `any` sin `@hybrid`, variables dinámicas o `eval`), imprime diagnósticos estilizados detallando la línea exacta y sugerencias de corrección.
* **Opciones**:
  * `--json`: Imprime todos los diagnósticos encontrados en un objeto estructurado JSON en la salida estándar. Esta opción es la que consumen las herramientas de análisis estático (como el plugin de ESLint).

---

### 2. `explain`
Muestra el plan de compilación detallado para un archivo TypeScript.

```bash
rustify explain src/main.ts
```

* **Utilidad**: Imprime en consola un desglose de las declaraciones de tipo reconocidas, cómo se traducirá cada sentencia y el código Rust resultante formateado.
* **Opciones**:
  * `--json`: Retorna toda la Representación Intermedia (IR) tipada y resuelta del compilador en formato JSON.

---

### 3. `compile`
Transpila el código fuente TypeScript a Rust nativo.

```bash
rustify compile src/main.ts --out dist-rust
```

* **Opciones**:
  * `--cargo`: (Recomendado) Envuelve el código generado en un proyecto Cargo estándar completo (crea `Cargo.toml`, estructura `src/main.rs`, y añade dependencias automáticas como `rustify-runtime` si utilizas timers asíncronos o APIs JSON).
  * `--no-cargo`: Genera un único archivo de código Rust `.rs` aislado sin crear directorios adicionales de configuración.
  * `--mode <hybrid|native>`: Controla la política de compatibilidad. Por defecto es `native`. Si se configura en `hybrid`, permite la presencia de funciones marcadas con `/** @hybrid */` delegando su ejecución a Node.js en tiempo de ejecución.

---

### 4. `init`
Inicializa un proyecto estructurado de Rustify en el directorio provisto.

```bash
rustify init mi-proyecto-rustify
```

Crea la siguiente estructura de archivos iniciales:
* `rustify.json` (Archivo de configuración global)
* `src/main.ts` (Archivo de entrada del código)
* `.gitignore` (Configuración de control de versiones)

---

## Configuración del Proyecto (`rustify.json`)

Para evitar pasar los argumentos a la CLI en cada invocación, puedes configurar las opciones por defecto en un archivo `rustify.json` en la raíz de tu proyecto:

```json
{
  "entry": "src/main.ts",
  "out": "dist-rust",
  "cargo": true,
  "package_name": "mi-proyecto-rustify",
  "mode": "native"
}
```

Si ejecutas los comandos sin parámetros dentro de un directorio con este archivo, la CLI lo resolverá automáticamente:
```bash
rustify compile
rustify check
```
