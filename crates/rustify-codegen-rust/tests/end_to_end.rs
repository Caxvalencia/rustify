#[test]
fn compiles_plan_example_to_rust() {
    let source = "type User = { id: number; name: string; active: boolean }\n\
                  function greet(user: User): string { return `Hola ${user.name}` }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub struct User"));
    assert!(rust.contains("pub id: f64"));
    assert!(rust.contains("format!(\"Hola {}\", user.name)"));
}
