import { appName, timeoutMs, maxConnections } from "./globals";

export function runDemo(): void {
  console.log("Aplicación: " + appName);
  console.log(`Timeout: ${timeoutMs}ms`);
  console.log(`Conexiones Máximas: ${maxConnections}`);
}
