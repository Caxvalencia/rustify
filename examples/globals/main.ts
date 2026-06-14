import { getAppName, getMaxConnections, getTimeoutMs } from "./globals";

export function runDemo(): void {
  console.log("Aplicación: " + getAppName());
  console.log("Timeout: " + getTimeoutMs() + "ms");
  console.log("Conexiones Máximas: " + getMaxConnections());
}
