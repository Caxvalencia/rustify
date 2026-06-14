# Plugin de ESLint (`eslint-plugin-rustify`) — Rustify

<p align="center">
  <img src="../assets/icon-eslint.png" alt="Rustify ESLint Icon" width="160" />
</p>

El paquete `eslint-plugin-rustify` integra el sistema de análisis estático del compilador dentro del ecosistema de JavaScript/Node.js. Permite a los desarrolladores recibir alertas rápidas directamente en su flujo tradicional de Node.js/CI sin necesidad de ejecutar el compilador de Cargo de forma manual.

---

## Cómo Funciona

Para evitar la duplicación de código e inconsistencias en las reglas, el plugin de ESLint no reimplementa el parser o el type-checker en JavaScript. En su lugar:
1. Invocará automáticamente al ejecutable `rustify check [archivo] --json` en segundo plano.
2. Analiza los diagnósticos estructurados en formato JSON retornados por el analizador semántico en Rust.
3. Traduce esos diagnósticos en reportes estándar de ESLint con el archivo, la línea, la columna y los mensajes de error correspondientes.

---

## Instalación

Instala el plugin localmente en tu proyecto Node.js:

```bash
npm install eslint-plugin-rustify --save-dev
```

---

## Configuración (ESLint v9+ Flat Config)

Crea o edita tu archivo de configuración `eslint.config.js` en la raíz de tu proyecto para importar e incorporar el plugin:

```javascript
import rustify from "eslint-plugin-rustify";

export default [
  // Carga las reglas recomendadas de compatibilidad de Rustify
  rustify.configs["flat/recommended"]
];
```

### Configuración con versiones previas (ESLint v8)

Si aún utilizas el formato anterior `.eslintrc.json`, puedes configurarlo así:

```json
{
  "plugins": ["rustify"],
  "extends": ["plugin:rustify/recommended"]
}
```

---

## Reglas Principales de Análisis

El plugin expone las siguientes reglas de validación en tu código TypeScript:

* **`rustify/no-any`**: Impide el uso del tipo `any` en funciones normales, sugiriendo cambiarlo por tipos primitivos estrictos o agregar la anotación `/** @hybrid */`.
* **`rustify/no-unknown`**: Bloquea el tipo dinámico `unknown`.
* **`rustify/no-eval`**: Prohíbe llamadas a la función global `eval()`.
* **`rustify/explicit-return-type`**: Obliga a que todas las funciones tengan un tipo de retorno definido explícitamente (ej: `fn(): void` o `fn(): string`).
* **`rustify/no-dynamic-object`**: Reporta la creación de propiedades dinámicas en objetos no tipados que no tengan su respectiva interfaz o struct en Rustify.
* **`rustify/no-unsupported-union`**: Reporta unions complejas. Solo se admiten unions que representen nulabilidad (ej: `T | null` o `T | undefined`).
