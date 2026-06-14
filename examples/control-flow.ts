function sum(values: number[]): number {
  let total: number = 0
  for (const value of values) {
    if (value === 0) {
      continue
    }
    total = total + value
    if (total > 100) {
      break
    }
  }
  return total
}

function describe(value: number): string {
  if (value > 0) {
    return "positive"
  } else {
    return "zero or negative"
  }
}
