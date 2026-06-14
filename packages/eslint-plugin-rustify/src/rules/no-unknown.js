import { problemRule } from "./helpers.js";

export default problemRule(
  "Disallow TypeScript `unknown`, which requires dynamic narrowing.",
  { forbidden: "`unknown` is not supported by Rustify. Use a concrete type." },
  (context) => ({
    TSUnknownKeyword(node) {
      context.report({ node, messageId: "forbidden" });
    }
  })
);
