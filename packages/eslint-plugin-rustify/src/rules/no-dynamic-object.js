import { problemRule } from "./helpers.js";

export default problemRule(
  "Disallow dynamic object shapes and prototype mutation.",
  {
    computed: "Dynamic property assignment is not supported by Rustify.",
    deletion: "Dynamic property deletion is not supported by Rustify.",
    index: "Dynamic index signatures are not supported by Rustify.",
    monkeyPatch: "Monkey patching with Object.defineProperty is not supported by Rustify.",
    prototype: "Prototype mutation is not supported by Rustify."
  },
  (context) => ({
    AssignmentExpression(node) {
      if (node.left.type === "MemberExpression" && node.left.computed) {
        context.report({ node: node.left, messageId: "computed" });
      }
    },
    UnaryExpression(node) {
      if (node.operator === "delete") context.report({ node, messageId: "deletion" });
    },
    MemberExpression(node) {
      if (node.property.type === "Identifier" && node.property.name === "prototype") {
        context.report({ node, messageId: "prototype" });
      }
    },
    TSIndexSignature(node) {
      context.report({ node, messageId: "index" });
    },
    CallExpression(node) {
      if (
        node.callee.type === "MemberExpression" &&
        node.callee.object.type === "Identifier" &&
        node.callee.object.name === "Object" &&
        node.callee.property.type === "Identifier" &&
        node.callee.property.name === "setPrototypeOf"
      ) {
        context.report({ node, messageId: "prototype" });
      }
      if (
        node.callee.type === "MemberExpression" &&
        node.callee.object.type === "Identifier" &&
        node.callee.object.name === "Object" &&
        node.callee.property.type === "Identifier" &&
        node.callee.property.name === "defineProperty"
      ) {
        context.report({ node, messageId: "monkeyPatch" });
      }
    }
  })
);
