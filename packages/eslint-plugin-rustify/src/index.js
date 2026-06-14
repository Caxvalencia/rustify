import parser from "@typescript-eslint/parser";
import rustifyDiagnostics from "./rules/rustify-diagnostics.js";

const rules = {
  "rustify-diagnostics": rustifyDiagnostics
};

const recommendedRules = {
  "rustify/rustify-diagnostics": "error"
};

const plugin = {
  meta: { name: "eslint-plugin-rustify", version: "0.1.0" },
  rules,
  configs: {}
};

plugin.configs.recommended = {
  parser: "@typescript-eslint/parser",
  plugins: ["rustify"],
  rules: recommendedRules
};

plugin.configs["flat/recommended"] = {
  files: ["**/*.ts", "**/*.tsx"],
  plugins: { rustify: plugin },
  languageOptions: { parser },
  rules: recommendedRules
};

export { rules };
export default plugin;
