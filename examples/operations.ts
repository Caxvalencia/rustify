export function enabled(left: boolean, right: boolean): boolean {
  return !left || right
}

export function remainder(value: number): number {
  return value % 2
}

export function join(left: string, right: string): string {
  return left + right
}

export function label(enabled: boolean): string {
  return enabled ? "enabled" : "disabled"
}

export function maybeLabel(enabled: boolean): string | null {
  return enabled ? "enabled" : null
}

export function bounded(value: number, limit: number): number {
  return Math.min(Math.max(value, 0), limit)
}

export function squared(value: number): number {
  return Math.pow(value, 2)
}
