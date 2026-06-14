import parser from "@typescript-eslint/parser";
import explicitParamTypes from "./rules/explicit-param-types.js";
import explicitReturnType from "./rules/explicit-return-type.js";
import noAny from "./rules/no-any.js";
import noDynamicObject from "./rules/no-dynamic-object.js";
import noEval from "./rules/no-eval.js";
import noUnknown from "./rules/no-unknown.js";
import noUnsupportedUnion from "./rules/no-unsupported-union.js";
import noUnsupportedSyntax from "./rules/no-unsupported-syntax.js";

const rules = {
  "no-any": noAny,
  "no-unknown": noUnknown,
  "no-eval": noEval,
  "no-dynamic-object": noDynamicObject,
  "no-unsupported-union": noUnsupportedUnion,
  "no-unsupported-syntax": noUnsupportedSyntax,
  "explicit-return-type": explicitReturnType,
  "explicit-param-types": explicitParamTypes
};

const recommendedRules = Object.fromEntries(
  Object.keys(rules).map((name) => [`rustify/${name}`, "error"])
);

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
