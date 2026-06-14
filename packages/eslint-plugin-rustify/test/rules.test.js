import parser from "@typescript-eslint/parser";
import { Linter, RuleTester } from "eslint";
import plugin, { rules } from "../src/index.js";

const tester = new RuleTester({
  languageOptions: {
    parser,
    parserOptions: { ecmaVersion: 2022, sourceType: "module" }
  }
});

tester.run("no-any", rules["no-any"], {
  valid: ["const value: string = 'ok'"],
  invalid: [
    {
      code: "const value: any = 'bad'",
      output: "const value: string = 'bad'",
      errors: [
        {
          messageId: "forbidden",
          suggestions: [{ messageId: "replaceWithInferred", output: "const value: string = 'bad'" }]
        }
      ]
    },
    {
      code: "const value: any = 1",
      output: "const value: number = 1",
      errors: [{ messageId: "forbidden", suggestions: 1 }]
    },
    {
      code: "const value: any = true",
      output: "const value: boolean = true",
      errors: [{ messageId: "forbidden", suggestions: 1 }]
    },
    {
      code: "function consume(value: any): void {}",
      errors: [{ messageId: "forbidden", suggestions: 0 }]
    },
    {
      code: "let value: any = 1",
      errors: [{ messageId: "forbidden", suggestions: 0 }]
    }
  ]
});

tester.run("no-unknown", rules["no-unknown"], {
  valid: ["const value: number = 1"],
  invalid: [{ code: "const value: unknown = 1", errors: [{ messageId: "forbidden" }] }]
});

tester.run("no-eval", rules["no-eval"], {
  valid: ["run(value)"],
  invalid: [
    { code: "eval(value)", errors: [{ messageId: "forbidden" }] },
    { code: "new Function('return 1')", errors: [{ messageId: "forbidden" }] }
  ]
});

tester.run("no-dynamic-object", rules["no-dynamic-object"], {
  valid: ["user.name = 'Ada'"],
  invalid: [
    { code: "user[key] = value", errors: [{ messageId: "computed" }] },
    { code: "delete user.name", errors: [{ messageId: "deletion" }] },
    { code: "type Values = { [key: string]: string }", errors: [{ messageId: "index" }] },
    { code: "Object.setPrototypeOf(user, base)", errors: [{ messageId: "prototype" }] },
    { code: "const proto = user.prototype", errors: [{ messageId: "prototype" }] },
    { code: "Object.defineProperty(user, 'name', {})", errors: [{ messageId: "monkeyPatch" }] }
  ]
});

tester.run("no-unsupported-union", rules["no-unsupported-union"], {
  valid: ["let value: string | null", "let value: string | undefined"],
  invalid: [
    { code: "let value: string | number", errors: [{ messageId: "forbidden" }] },
    { code: "let value: string | number | null", errors: [{ messageId: "forbidden" }] }
  ]
});

tester.run("no-unsupported-syntax", rules["no-unsupported-syntax"], {
  valid: ["import { user } from './user'; user.name"],
  invalid: [
    { code: "import value from 'package'", errors: [{ messageId: "externalImport" }] },
    { code: "function read() { return this.value }", errors: [{ messageId: "dynamicThis" }] },
    { code: "Reflect.getMetadata('x', value)", errors: [{ messageId: "reflect" }] },
    { code: "declare function external(): void", errors: [{ messageId: "ambient" }] },
    { code: "namespace Values { export const x = 1 }", errors: [{ messageId: "namespace" }] },
    { code: "const read = () => 1", errors: [{ messageId: "unsupported" }] },
    { code: "class User {}", errors: [{ messageId: "unsupported" }] },
    { code: "try { run() } catch {}", errors: [{ messageId: "unsupported" }] },
    { code: "function read() { return this }", errors: [{ messageId: "dynamicThis" }] }
  ]
});

tester.run("explicit-return-type", rules["explicit-return-type"], {
  valid: ["function greet(name: string): string { return name }"],
  invalid: [{ code: "function greet(name: string) { return name }", errors: [{ messageId: "missing" }] }]
});

tester.run("explicit-param-types", rules["explicit-param-types"], {
  valid: ["function greet(name: string): string { return name }"],
  invalid: [{ code: "function greet(name): string { return name }", errors: [{ messageId: "missing" }] }]
});

const linter = new Linter({ configType: "flat" });
const messages = linter.verify(
  "function unsafe(value: any) { return eval(value) }",
  [plugin.configs["flat/recommended"]],
  { filename: "input.ts" }
);

for (const ruleId of ["rustify/no-any", "rustify/no-eval", "rustify/explicit-return-type"]) {
  if (!messages.some((message) => message.ruleId === ruleId)) {
    throw new Error(`flat/recommended did not enable ${ruleId}`);
  }
}

const fixed = linter.verifyAndFix("const count: any = 1", [plugin.configs["flat/recommended"]], {
  filename: "input.ts"
});
if (!fixed.fixed || fixed.output !== "const count: number = 1") {
  throw new Error(`no-any did not apply its safe inferred autofix: ${fixed.output}`);
}

const unsafeFix = linter.verifyAndFix(
  "function consume(value: any): void {}",
  [plugin.configs["flat/recommended"]],
  { filename: "input.ts" }
);
if (unsafeFix.fixed) {
  throw new Error("no-any unexpectedly autofixed a parameter without a safe inferred type");
}
