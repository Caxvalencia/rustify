import { problemRule } from "./helpers.js";

export default problemRule(
  "Disallow TypeScript and JavaScript syntax that has no native Rustify representation.",
  {
    ambient: "Ambient declarations are not supported by Rustify.",
    decorator: "Decorators are not supported by Rustify.",
    dynamicThis: "Dynamic `this` access is not supported by Rustify.",
    externalImport: "Only relative Rustify module imports are supported.",
    namespace: "TypeScript namespaces and global augmentation are not supported.",
    reflect: "Reflect metadata and dynamic reflection are not supported by Rustify.",
    unsupported: "This syntax has no native Rustify representation."
  },
  (context) => ({
    Decorator(node) {
      context.report({ node, messageId: "decorator" });
    },
    ImportDeclaration(node) {
      if (!node.source.value.startsWith(".")) {
        context.report({ node, messageId: "externalImport" });
      }
    },
    MemberExpression(node) {
      if (node.object.type === "Identifier" && node.object.name === "Reflect") {
        context.report({ node, messageId: "reflect" });
      }
    },
    ThisExpression(node) {
      context.report({ node, messageId: "dynamicThis" });
    },
    TSDeclareFunction(node) {
      context.report({ node, messageId: "ambient" });
    },
    TSModuleDeclaration(node) {
      context.report({ node, messageId: "namespace" });
    },
    ArrowFunctionExpression(node) {
      context.report({ node, messageId: "unsupported" });
    },
    FunctionExpression(node) {
      context.report({ node, messageId: "unsupported" });
    },
    ClassDeclaration(node) {
      context.report({ node, messageId: "unsupported" });
    },
    ClassExpression(node) {
      context.report({ node, messageId: "unsupported" });
    },
    SwitchStatement(node) {
      context.report({ node, messageId: "unsupported" });
    },
    TryStatement(node) {
      context.report({ node, messageId: "unsupported" });
    },
    ThrowStatement(node) {
      context.report({ node, messageId: "unsupported" });
    }
  })
);
