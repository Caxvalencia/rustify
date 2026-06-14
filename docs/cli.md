# Interfaz de Línea de Comandos (CLI)

El crate `rustify-cli` provee la herramienta ejecutable para interactuar con el compilador.

## Comandos Disponibles

### 1. `check`
Verifica un archivo de TypeScript contra el subconjunto de reglas de Rustify.

```bash
rustify check src/main.ts
```

Opciones:
- `--json`: Retorna los diagnósticos encontrados en un formato estructurado JSON. Muy útil para herramientas de linter e integración en editores.

---

### 2. `explain`
Muestra el plan de compilación para un archivo TypeScript. Imprime las firmas Rust detectadas, cómo se traducirá cada sentencia y el código Rust resultante.

```bash
rustify explain src/main.ts
```

Opciones:
- `--json`: Imprime la representación intermedia (IR) tipada completa en formato JSON.

---

### 3. `compile`
Transpila el código fuente TypeScript a Rust.

```bash
rustify compile src/main.ts --out dist-rust
```

Opciones:
- `--cargo`: Crea un proyecto Cargo completo con una estructura estándar, incluyendo `Cargo.toml` y empaquetando el runtime `rustify-runtime` si se utiliza JSON o timers asíncronos.
- `--no-cargo`: Genera un único archivo de código Rust `.rs` sin envolverlo en un proyecto Cargo.
- `--mode <hybrid|native>`: Elige el modo de compilación (ver sección "Modo Híbrido").

---

### 4. `init`
Inicializa un nuevo proyecto Rustify en el directorio especificado, creando una plantilla de configuración `rustify.json` y el directorio de fuentes inicial.

```bash
rustify init my-project
```

---

## Configuración del Proyecto (`rustify.json`)

En lugar de especificar argumentos en la CLI cada vez, puedes definir un archivo `rustify.json` en la raíz de tu proyecto:

```json
{
  "entry": "src/main.ts",
  "out": "dist-rust",
  "cargo": true,
  "package_name": "my-rustify-app",
  "mode": "native"
}
```

---

## Modo Híbrido

Cuando el modo se establece en `"hybrid"`, el compilador intentará compilar el código TypeScript a Rust nativo primero. Si encuentra características dinámicas no soportadas por la especificación de Rustify:
1. Genera un directorio `fallback` con el código TypeScript.
2. Escribe una configuración de inicio en `package.json` para ejecutar el código usando el motor V8 de Node.js mediante `--experimental-transform-types`.
3. Registra la decisión en `rustify-hybrid.json`.
