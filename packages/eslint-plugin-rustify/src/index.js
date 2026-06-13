const forbidden = {
  "no-any": { pattern: /\bany\b/g, message: "`any` is not supported by Rustify." },
  "no-unknown": { pattern: /\bunknown\b/g, message: "`unknown` is not supported by Rustify." },
  "no-eval": { pattern: /\beval\s*\(/g, message: "`eval` is not supported by Rustify." },
  "no-unsupported-union": {
    pattern: /\b(string|number|boolean)\s*\|\s*(string|number|boolean)\b/g,
    message: "Only nullable unions are supported by Rustify."
  }
};

const rules = Object.fromEntries(
  Object.entries(forbidden).map(([name, rule]) => [
    name,
    {
      meta: { type: "problem", docs: { description: rule.message }, schema: [] },
      create(context) {
        return {
          Program(node) {
            const source = context.sourceCode.getText();
            for (const match of source.matchAll(rule.pattern)) {
              context.report({ node, loc: context.sourceCode.getLocFromIndex(match.index), message: rule.message });
            }
          }
        };
      }
    }
  ])
);

export default {
  rules,
  configs: {
    recommended: {
      plugins: ["rustify"],
      rules: Object.fromEntries(Object.keys(rules).map((name) => [`rustify/${name}`, "error"]))
    }
  }
};

