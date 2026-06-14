export function problemRule(description, messages, create, options = {}) {
  return {
    meta: {
      type: "problem",
      docs: { description, recommended: true },
      messages,
      schema: [],
      ...options
    },
    create
  };
}

export function unwrapParameter(parameter) {
  if (parameter.type === "AssignmentPattern") return parameter.left;
  if (parameter.type === "RestElement") return parameter.argument;
  return parameter;
}

export function functionName(node) {
  if (node.id?.name) return node.id.name;
  if (node.parent?.type === "VariableDeclarator" && node.parent.id.type === "Identifier") {
    return node.parent.id.name;
  }
  return "<anonymous>";
}
