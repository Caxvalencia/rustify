import { User, greet } from "./models"

export function welcome(user: User): string {
  return greet(user)
}
