export enum Status {
  Active,
  Inactive
}

export function current(): Status {
  return Status.Active
}

export function maybeName(enabled: boolean): string | null {
  if (enabled) {
    return "Rustify"
  }
  return null
}

export function announce(status: Status): void {
  console.log("Current status:", status)
}
