export type Address = {
  city: string
}

export type User = {
  name: string
  nickname?: string
  address: Address
}

export function buildUser(): User {
  return {
    name: "Ada",
    address: {
      city: "London"
    }
  }
}

export function hasNickname(user: User): boolean {
  return user.nickname.isSome()
}

export function displayNickname(user: User, fallback: string): string {
  return user.nickname.unwrapOr(fallback)
}
