use rustify_analyzer::analyze;
use rustify_parser::parse;
use std::time::Instant;

fn main() {
    println!("=== RUSTIFY COMPILER BENCHMARK ===");

    let source_code = r#"
type User = {
  id: number;
  name: string;
  active: boolean;
};

enum Role {
  Admin,
  User,
  Guest
}

function compute_score(base: number, multiplier: number): number {
  let score = base * multiplier;
  if (score > 100) {
    return 100;
  }
  return score;
}

export function process_users(users: User[]): string[] {
  let results: string[] = [];
  for (let user of users) {
    if (user.active) {
      results.push("User: " + user.name);
    }
  }
  return results;
}
"#;

    let iterations = 1000;
    println!(
        "Ejecutando {} iteraciones de parser + analyzer...",
        iterations
    );

    let start = Instant::now();
    for _ in 0..iterations {
        let program = parse(source_code).unwrap();
        let _analysis = analyze(&program);
    }
    let duration = start.elapsed();
    let avg = duration / iterations;

    println!("Tiempo total: {:?}", duration);
    println!("Tiempo promedio por iteración: {:?}", avg);
}
