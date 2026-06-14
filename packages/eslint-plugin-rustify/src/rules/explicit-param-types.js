import { functionName, problemRule, unwrapParameter } from "./helpers.js";

export default problemRule(
  "Require explicit function parameter types.",
  { missing: "Parameter `{{name}}` in `{{functionName}}` requires an explicit type." },
  (context) => {
    const check = (node) => {
      for (const parameter of node.params) {
        const target = unwrapParameter(parameter);
        if (!target.typeAnnotation) {
          context.report({
            node: target,
            messageId: "missing",
            data: {
              name: target.type === "Identifier" ? target.name : "<pattern>",
              functionName: functionName(node)
            }
          });
        }
      }
    };
    return {
      FunctionDeclaration: check,
      FunctionExpression: check,
      ArrowFunctionExpression: check
    };
  }
);
