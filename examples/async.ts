export async function loadMessage(): Promise<string> {
  return "ready"
}

export async function relayMessage(): Promise<string> {
  return await loadMessage()
}
