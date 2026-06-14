import parser from "@typescript-eslint/parser";
import { Linter, RuleTester } from "eslint";
import plugin, { rules } from "../src/index.js";
import fs from "fs";
import path from "path";

// Asegurar directorio temporal
const tempDir = path.resolve(process.cwd(), "temp-test-files");
if (!fs.existsSync(tempDir)) {
  fs.mkdirSync(tempDir);
}

// Limpieza de archivos temporales al terminar
process.on("exit", () => {
  try {
    fs.rmSync(tempDir, { recursive: true, force: true });
  } catch (_) {}
});

// Sobrescribimos RuleTester para crear el archivo físico correspondiente al código antes de que ESLint lo verifique,
// ya que RuleTester solo pasa el código como string y no crea archivos físicos por defecto.
class PhysicalRuleTester extends RuleTester {
  run(name, rule, tests) {
    const mapTest = (testCase) => {
      // Usamos una ruta relativa para que coincida con el glob de eslint (**/*.ts)
      const relativePath = `temp-test-files/test_${Math.random().toString(36).substring(7)}.ts`;
      const fullPath = path.resolve(process.cwd(), relativePath);
      fs.writeFileSync(fullPath, testCase.code, "utf8");
      return {
        ...testCase,
        filename: relativePath // ESLint flat config requiere ruta relativa para que coincida con los globs
      };
    };

    const newTests = {
      valid: tests.valid.map((t) => (typeof t === "string" ? { code: t } : t)).map(mapTest),
      invalid: tests.invalid.map(mapTest)
    };

    super.run(name, rule, newTests);
  }
}

// Usamos el parser de typescript para ESLint en el tester
const tester = new PhysicalRuleTester({
  languageOptions: {
    parser,
    parserOptions: { ecmaVersion: 2022, sourceType: "module" }
  }
});

tester.run("rustify-diagnostics", rules["rustify-diagnostics"], {
  valid: [
    "function greet(name: string): string { return name }"
  ],
  invalid: [
    {
      code: "function greet(name: any): string { return name }",
      errors: [
        {
          message: /\[SFT013\] type `any` is not supported by Rustify/
        },
        {
          message: /\[SFT033\] function `greet` returns `any`, expected `string`/
        },
        {
          message: /\[SFT001\] `any` is not supported by Rustify\./
        }
      ]
    },
    {
      code: "function greet(name: string): string { eval(name); return name }",
      errors: [
        {
          message: /\[SFT031\] unknown function `eval`/
        },
        {
          message: /\[SFT003\] `eval` cannot be compiled to native Rust\./
        }
      ]
    }
  ]
});

// Test de flat/recommended config
const linter = new Linter({ configType: "flat" });
const tempFileForFlat = "temp-test-files/flat_test.ts";
fs.writeFileSync(path.resolve(process.cwd(), tempFileForFlat), "function unsafe(value: any) { return eval(value) }", "utf8");

const messages = linter.verify(
  "function unsafe(value: any) { return eval(value) }",
  [plugin.configs["flat/recommended"]],
  { filename: tempFileForFlat }
);

if (!messages.some((message) => message.ruleId === "rustify/rustify-diagnostics")) {
  throw new Error("flat/recommended did not enable rustify/rustify-diagnostics");
}
