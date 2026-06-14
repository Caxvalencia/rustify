import { functionName, problemRule } from "./helpers.js";

export default problemRule(
  "Require explicit function return types.",
  { missing: "Function `{{name}}` requires an explicit return type for Rustify." },
  (context) => {
    const check = (node) => {
      if (!node.returnType) {
        context.report({ node, messageId: "missing", data: { name: functionName(node) } });
      }
    };
    return {
      FunctionDeclaration: check,
      FunctionExpression: check,
      ArrowFunctionExpression: check
    };
  }
);
