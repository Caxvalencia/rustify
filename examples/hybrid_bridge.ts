// examples/hybrid_bridge.ts

type User = {
  name: string;
  role: string;
};

// Esta función es 100% nativa y se compilará directamente a Rust
pub function add(a: number, b: number): number {
  return a + b;
}

// Esta función es híbrida y se ejecutará de forma delegada en Node.js
/** @hybrid */
pub function greet_dynamic(user: any): string {
  return "Hola " + user.name + " con rol " + user.role;
}

// Función que coordina la demostración
pub function demo(): void {
  const sum = add(10, 20);
  console.log("Suma nativa compilada en Rust: " + sum);

  const mockUser: User = {
    name: "Antigravity",
    role: "AI Developer"
  };

  // Esto invocará a la función dinámica híbrida a través del bridge
  const greeting = greet_dynamic(mockUser);
  console.log("Saludo dinamico de Node.js: " + greeting);
}
