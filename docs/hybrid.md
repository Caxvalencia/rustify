# Bridge de Interoperabilidad Híbrida en Rustify

Rustify está diseñado para compilar TypeScript a código nativo en Rust. Sin embargo, TypeScript y JavaScript son lenguajes extremadamente dinámicos, y ciertas características complejas (como tipos `any` o evaluación dinámica) no se pueden transpolar de manera directa y segura a Rust nativo.

Para solucionar esto, Rustify incluye un **Bridge de Interoperabilidad Híbrida** (Modo Híbrido) que permite delegar selectivamente el cuerpo de funciones específicas a un motor JavaScript/V8 administrado en tiempo de ejecución.

---

## Cómo Funciona

Cuando compilas tu proyecto en el modo híbrido (`--mode hybrid`):

1. **Detección**: El compilador busca funciones que tengan la anotación javadoc `/** @hybrid */` inmediatamente antes de su declaración.
2. **Generación de Código de Rust**: Para cada función marcada con `@hybrid`, el compilador:
   - Valida que los parámetros y el retorno sean tipos básicos de Rustify (o `any`, que se mapeará automáticamente a `JsonValue`).
   - Reemplaza el cuerpo de la función en Rust por una llamada al bridge:
     ```rust
     rustify_runtime::call_js_fallback(source_path, func_name, &[arguments]).unwrap()
     ```
3. **Copia de Fuentes**: El compilador copia de forma transparente todos los archivos TypeScript del proyecto a un directorio local llamado `fallback/`.
4. **Ejecución en Caliente**: En tiempo de ejecución, la función de Rust inicia un proceso ligero de Node.js, carga dinámicamente el módulo correspondiente en `fallback/` usando `--experimental-transform-types`, y ejecuta la función original en JS. Los datos se intercambian de forma transparente y síncrona mediante JSON sobre stdio/IPC.

---

## Ejemplo Práctico

Crea un archivo llamado `src/main.ts`:

```ts
type User = {
  name: string;
  role: string;
};

// Esta función es 100% nativa y compatible. Se compilará a código Rust nativo ultra rápido.
pub function add(a: number, b: number): number {
  return a + b;
}

// Esta función usa el tipo prohibido `any` y no se puede compilar a Rust nativo.
// Al marcarla con @hybrid, el compilador delegará su ejecución a Node.js de forma transparente.
/** @hybrid */
pub function greet_dynamic(user: any): string {
  return "Hola " + user.name + " con rol " + user.role;
}

pub function demo(): void {
  const sum = add(10, 20);
  console.log("Resultado nativo: " + sum);

  const mockUser: User = {
    name: "Antigravity",
    role: "Developer"
  };

  const message = greet_dynamic(mockUser);
  console.log("Resultado híbrido: " + message);
}
```

### Compilación

Compila el proyecto especificando el modo híbrido y habilitando un proyecto Cargo completo:

```bash
rustify compile src/main.ts --out dist --cargo --mode hybrid
```

El código de Rust generado para `greet_dynamic` se verá así:

```rust
pub fn greet_dynamic(user: serde_json::Value) -> String {
    rustify_runtime::call_js_fallback("src/main.ts", "greet_dynamic", &[serde_json::json!(user)]).unwrap()
}
```

Esto te permite mantener lo mejor de ambos mundos: rendimiento nativo de Rust para la lógica estructurada de tu aplicación, y total compatibilidad con TypeScript dinámico para integraciones complejas.
