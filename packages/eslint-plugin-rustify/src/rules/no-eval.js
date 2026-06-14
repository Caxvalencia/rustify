import { problemRule } from "./helpers.js";

export default problemRule(
  "Disallow dynamic code evaluation.",
  { forbidden: "`eval` cannot be compiled to native Rust." },
  (context) => ({
    CallExpression(node) {
      if (node.callee.type === "Identifier" && node.callee.name === "eval") {
        context.report({ node, messageId: "forbidden" });
      }
    },
    NewExpression(node) {
      if (node.callee.type === "Identifier" && node.callee.name === "Function") {
        context.report({ node, messageId: "forbidden" });
      }
    }
  })
);
