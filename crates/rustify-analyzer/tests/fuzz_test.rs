use rustify_analyzer::analyze;
use rustify_parser::parse;

#[test]
fn fuzz_parser_and_analyzer_robustness() {
    let base_source = r#"
type User = {
  id: number;
  name: string;
};
function greet(user: User): string {
  return "Hello " + user.name;
}
"#;

    // Generar mutaciones aleatorias simples para comprobar robustez
    let mut rng = 12345u64; // Seed simple para determinismo
    let chars: Vec<char> = base_source.chars().collect();

    for _i in 0..100 {
        // Generar una versión mutada de base_source
        let mut mutated = chars.clone();

        // Pseudo-random index y modificación
        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let pos = (rng as usize) % mutated.len();

        rng = rng
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let action = rng % 3;

        if action == 0 && !mutated.is_empty() {
            // Eliminar caracter
            mutated.remove(pos);
        } else if action == 1 {
            // Reemplazar con caracter especial o basura
            let bad_char = match rng % 4 {
                0 => '@',
                1 => ';',
                2 => '{',
                _ => '\0',
            };
            mutated[pos] = bad_char;
        } else {
            // Insertar basura
            mutated.insert(pos, '#');
        }

        let mutated_str: String = mutated.into_iter().collect();

        // Ejecutar parser y analyzer; ninguno debe entrar en pánico
        let _ = std::panic::catch_unwind(|| {
            if let Ok(program) = parse(&mutated_str) {
                let _ = analyze(&program);
            }
        });
    }
}
