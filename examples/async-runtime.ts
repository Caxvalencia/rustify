export async function pause(milliseconds: number): Promise<void> {
  await Rustify.sleep(milliseconds)
}
