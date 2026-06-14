import { problemRule } from "./helpers.js";

function inferredConstType(node) {
  const annotation = node.parent;
  const identifier = annotation?.parent;
  const declarator = identifier?.parent;
  const declaration = declarator?.parent;
  if (
    annotation?.type !== "TSTypeAnnotation" ||
    identifier?.type !== "Identifier" ||
    declarator?.type !== "VariableDeclarator" ||
    declaration?.type !== "VariableDeclaration" ||
    declaration.kind !== "const"
  ) {
    return null;
  }

  const initializer = declarator.init;
  if (initializer?.type === "Literal") {
    if (typeof initializer.value === "string") return "string";
    if (typeof initializer.value === "number") return "number";
    if (typeof initializer.value === "boolean") return "boolean";
  }
  if (initializer?.type === "TemplateLiteral") return "string";
  return null;
}

export default problemRule(
  "Disallow TypeScript `any`, which cannot be compiled safely to Rust.",
  {
    forbidden: "`any` is not supported by Rustify. Use a concrete type.",
    replaceWithInferred: "Replace `any` with the inferred `{{type}}` type."
  },
  (context) => ({
    TSAnyKeyword(node) {
      const inferred = inferredConstType(node);
      context.report({
        node,
        messageId: "forbidden",
        ...(inferred
          ? {
              fix: (fixer) => fixer.replaceText(node, inferred),
              suggest: [
                {
                  messageId: "replaceWithInferred",
                  data: { type: inferred },
                  fix: (fixer) => fixer.replaceText(node, inferred)
                }
              ]
            }
          : {})
      });
    }
  }),
  { fixable: "code", hasSuggestions: true }
);
