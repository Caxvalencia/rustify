use rustify_analyzer::analyze;

#[test]
fn lowers_valid_typescript_to_typed_ir() {
    let source = "type User = { id: number; tags: string[]; name?: string }\n\
                  function greet(user: User): string { return `Hello ${user.id}` }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert_eq!(ir.structs.len(), 1);
    assert_eq!(ir.functions.len(), 1);
}
