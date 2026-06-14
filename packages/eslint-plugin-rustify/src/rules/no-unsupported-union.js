import { problemRule } from "./helpers.js";

const nullable = new Set(["TSNullKeyword", "TSUndefinedKeyword"]);

export default problemRule(
  "Allow only nullable TypeScript unions.",
  { forbidden: "Rustify supports only `T | null` and `T | undefined` unions." },
  (context) => ({
    TSUnionType(node) {
      const concrete = node.types.filter((type) => !nullable.has(type.type));
      const optional = node.types.filter((type) => nullable.has(type.type));
      if (concrete.length !== 1 || optional.length === 0) {
        context.report({ node, messageId: "forbidden" });
      }
    }
  })
);
