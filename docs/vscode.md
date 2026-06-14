# Extensión de VS Code (`vscode-rustify`) — Rustify

<p align="center">
  <img src="../assets/icon-vscode.png" alt="Rustify VS Code Icon" width="160" />
</p>

La extensión `vscode-rustify` integra la experiencia de desarrollo del compilador de Rustify directamente dentro del editor de código Visual Studio Code. Gestiona de manera automática el servidor de lenguaje (`rustify-lsp`) y ofrece comandos interactivos para previsualizar el código Rust generado.

---

## Características

* **Cliente LSP Automático**: Descubre y arranca el binario `rustify-lsp` en segundo plano cuando abres un archivo TypeScript.
* **Diagnósticos Inline**: Muestra subrayados de error de color rojo y advertencias al instante si el código infringe las reglas de Rustify.
* **Vista Previa de Traducción en Tiempo Real**: Comando interactivo que abre una vista en paralelo mostrando el código Rust generado a partir de tu archivo TypeScript activo.
* **Diagnósticos de Importaciones**: Detecta si importas recursos privados o si existen dependencias circulares prohibidas entre módulos de tu espacio de trabajo.

---

## Construcción e Instalación de la Extensión

Para compilar y empaquetar la extensión localmente, asegúrate de tener instalado Node.js. Ejecuta la tarea provista en el Makefile:

```bash
make package
```

Este comando invocará las herramientas de empaquetado de extensiones y generará el instalador oficial `.vsix` en:
```bash
dist-packages/vscode-rustify-0.1.0.vsix
```

### Cómo Instalarla en VS Code

Puedes instalar el archivo `.vsix` generado directamente desde tu terminal:

```bash
code --install-extension dist-packages/vscode-rustify-0.1.0.vsix
```

O bien, de manera interactiva dentro de VS Code:
1. Abre la pestaña **Extensiones** (`Ctrl+Shift+X` o `Cmd+Shift+X`).
2. Haz clic en el menú de tres puntos (`...`) en la esquina superior derecha del panel de extensiones.
3. Selecciona **Install from VSIX...** y elige el archivo `vscode-rustify-0.1.0.vsix` generado.

---

## Comandos Disponibles en VS Code

Puedes invocar estos comandos abriendo la Paleta de Comandos (`Ctrl+Shift+P` o `Cmd+Shift+P`):

* **`Rustify: Preview Generated Rust Code`** (`rustify.preview`): Abre un panel dividido a la derecha que muestra en tiempo de ejecución cómo se traduce tu archivo TypeScript a Rust. A diferencia de ejecutar la CLI manual, este comando consulta directamente al LSP y muestra la previsualización del buffer actual del editor, **incluso con los cambios que aún no has guardado**.
* **`Rustify: Run Syntax Checks`** (`rustify.check`): Ejecuta el comando de validación del linter para el archivo activo dentro de una terminal integrada de VS Code.
