type User = {
  id: number
  name: string
  active: boolean
  nickname?: string
}

enum Status {
  Active,
  Inactive
}

function greet(user: User): string {
  return `Hola ${user.name}`
}

