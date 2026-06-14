export function containsName(names: string[], target: string): boolean {
  return names.includes(target)
}

export function matchesPrefixOrSuffix(value: string, query: string): boolean {
  return value.startsWith(query) || value.endsWith(query)
}

export function containsText(value: string, query: string): boolean {
  return value.includes(query)
}

export function normalizeText(value: string): string {
  return value.trim().toLowerCase()
}

export function shout(value: string): string {
  return value.trim().toUpperCase()
}

export function appendName(names: string[], name: string): string[] {
  let result: string[] = names
  result.push(name)
  return result
}

export function takeLastName(names: string[]): string | null {
  let result: string[] = names
  return result.pop()
}

export function joinNames(names: string[], separator: string): string {
  return names.join(separator)
}

export function firstName(names: string[]): string | null {
  return names[0]
}

export function nameAt(names: string[], index: number): string | null {
  return names[index]
}

export function hasFirstName(names: string[]): boolean {
  return names[0].isSome()
}

export function firstNameOr(names: string[], fallback: string): string {
  return names[0].unwrapOr(fallback)
}
