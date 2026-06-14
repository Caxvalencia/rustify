export function parseDocument(input: string): Result<JsonValue, string> {
  return JSON.parse(input)
}

export function stringifyDocument(value: JsonValue): Result<string, string> {
  return JSON.stringify(value)
}

export function success(value: string): Result<string, string> {
  return Ok(value)
}

export function failure(message: string): Result<string, string> {
  return Err(message)
}

export function parses(input: string): boolean {
  return JSON.parse(input).isOk()
}

export function failed(value: Result<string, string>): boolean {
  return value.isErr()
}

export function valueOr(value: Result<string, string>, fallback: string): string {
  return value.unwrapOr(fallback)
}
