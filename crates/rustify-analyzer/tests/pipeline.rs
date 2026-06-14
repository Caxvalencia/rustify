use rustify_analyzer::{analyze, validate_module_scope};
use rustify_ir::{ExpressionKind, Statement, Type};

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

#[test]
fn rejects_semantic_type_errors() {
    let source = "type User = { name: string }\n\
                  function label(user: User): string { return user.missing }\n\
                  function add(value: number): number { const result: string = value; return result }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT035"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT032"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT033"), "{:?}", analysis.diagnostics);
}

#[test]
fn rejects_unknown_named_types_and_bad_arguments() {
    let source = "function consume(value: number): void { console.log(value) }\n\
                  function bad(value: Missing): void { consume(\"wrong\") }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT021")
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT034")
    );
}

#[test]
fn validates_nested_control_flow() {
    let source = "function invalid(value: number): number {\n\
                    if (value) { return \"wrong\" }\n\
                    for (const item of value) { console.log(item) }\n\
                    return value\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT038"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT039"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT033"), "{:?}", analysis.diagnostics);
}

#[test]
fn requires_non_void_functions_to_return_on_every_path() {
    let source = "function missing(value: boolean): string {\n\
                    if (value) { return \"yes\" }\n\
                  }\n\
                  function complete(value: boolean): string {\n\
                    if (value) { return \"yes\" } else { return \"no\" }\n\
                  }\n\
                  function trailing(value: boolean): string {\n\
                    if (value) { return \"yes\" }\n\
                    return \"no\"\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT063")
            .count(),
        1,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn rejects_names_that_collide_after_rust_normalization() {
    let source = "type User = { firstName: string; first_name: string }\n\
                  type user_record = { id: number }\n\
                  type UserRecord = { id: number }\n\
                  enum task_status { inProgress, in_progress }\n\
                  function loadValue(): number { return 1 }\n\
                  function load_value(): number { return 2 }\n\
                  function parameters(userName: string, user_name: string): void { return }\n\
                  function locals(): void {\n\
                    const localName: string = \"a\"\n\
                    const local_name: string = \"b\"\n\
                  }\n\
                  function loop(itemName: string, values: string[]): void {\n\
                    for (const item_name of values) { console.log(item_name) }\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT064")
            .count()
            >= 7,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn validates_and_lowers_loop_control_statements() {
    let valid = rustify_parser::parse(
        "function scan(values: number[]): void {
           for (const value of values) {
             if (value === 0) { continue }
             if (value > 10) { break }
           }
         }",
    )
    .unwrap();
    let analysis = analyze(&valid);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        &ir.functions[0].body[0],
        Statement::ForOf { body, .. }
            if matches!(&body[0], Statement::If { then_body, .. } if matches!(then_body[0], Statement::Continue))
                && matches!(&body[1], Statement::If { then_body, .. } if matches!(then_body[0], Statement::Break))
    ));

    let invalid = rustify_parser::parse("function stop(): void { break; continue }").unwrap();
    let analysis = analyze(&invalid);
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT051")
            .count(),
        2,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn produces_typed_structured_ir() {
    let source = "function sum(values: number[]): number {\n\
                    let total: number = 0\n\
                    for (const value of values) { total = total + value }\n\
                    return total\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let function = &analysis.ir.unwrap().functions[0];
    assert!(matches!(
        &function.body[0],
        Statement::Variable {
            mutable: true,
            ty: Type::F64,
            value,
            ..
        } if value.ty == Type::F64
    ));
    assert!(matches!(
        &function.body[1],
        Statement::ForOf { body, .. }
            if matches!(
                &body[0],
                Statement::Assignment { value, .. }
                    if matches!(value.kind, ExpressionKind::Binary { .. })
            )
    ));
}

#[test]
fn rejects_assignment_to_immutable_bindings() {
    let source = "function constBinding(): number {\n\
                    const value: number = 1\n\
                    value = 2\n\
                    return value\n\
                  }\n\
                  function parameter(value: number): number {\n\
                    value = 2\n\
                    return value\n\
                  }\n\
                  function mutableBinding(): number {\n\
                    let value: number = 1\n\
                    if (value > 0) { value = 2 }\n\
                    return value\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT055")
            .count(),
        2,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn rejects_unknown_identifiers_inside_templates_and_console_log() {
    let source = "function bad(): string { console.log(missing); return `Hi ${missing}` }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(!analysis.is_valid());
    assert!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT031")
            .count()
            >= 2
    );
}

#[test]
fn lowers_enum_variants_and_optional_coercions() {
    let source = "enum Status { Active, Inactive }\n\
                  function current(): Status { return Status.Active }\n\
                  function maybe(enabled: boolean): string | null {\n\
                    if (enabled) { return \"yes\" }\n\
                    return null\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if matches!(value.kind, ExpressionKind::EnumVariant { .. })
    ));
    assert!(matches!(
        ir.functions[1].body[0],
        Statement::If { ref then_body, .. }
            if matches!(
                then_body[0],
                Statement::Return(ref value)
                    if matches!(value.kind, ExpressionKind::Some(_))
            )
    ));
    assert!(matches!(
        ir.functions[1].body[1],
        Statement::Return(ref value) if matches!(value.kind, ExpressionKind::Null)
    ));
}

#[test]
fn validates_and_lowers_multi_argument_console_log() {
    let source = "function report(label: string, value: number, active: boolean): void {\n\
                    console.log(label, value, `active=${active}`)\n\
                    console.log()\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::ConsoleLog(ref values) if values.len() == 3
    ));
    assert!(matches!(
        ir.functions[0].body[1],
        Statement::ConsoleLog(ref values) if values.is_empty()
    ));

    let invalid = rustify_parser::parse(
        "type User = { name: string }\n\
         async function load(): Promise<string> { return \"ready\" }\n\
         function broken(label: string, user: User): void {\n\
           console.log(label, missing)\n\
           console.log(load())\n\
           console.log(`user=${user}`)\n\
         }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT031")
    );
    assert!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT059")
            .count()
            >= 2,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn reports_prohibited_syntax_with_shared_codes() {
    let source = "import { value } from \"package\"\n\
                  declare function external(): void\n\
                  namespace Global { export const value = 1 }\n\
                  function reflected(value: string): string { Reflect.get(value, \"x\"); return this.value }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT009"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT014"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT016"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT025"), "{:?}", analysis.diagnostics);
}

#[test]
fn prohibited_syntax_scanner_ignores_strings_and_comments() {
    let source = "function email(): string {\n\
                    // eval(value) @decorator Reflect.get(value)\n\
                    return \"user@example.com this.value\"\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
}

#[test]
fn distinguishes_array_types_from_dynamic_property_assignment() {
    let valid = rustify_parser::parse(
        "function values(): string[] { const items: string[] = []; return items }",
    )
    .unwrap();
    assert!(analyze(&valid).is_valid());

    let invalid = rustify_parser::parse(
        "function mutate(key: string): void { const values: string[] = []; values[key] = \"x\" }",
    )
    .unwrap();
    assert!(
        analyze(&invalid)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT018")
    );
}

#[test]
fn lowers_safe_json_and_result_apis() {
    let source = "function parse(input: string): Result<JsonValue, string> { return JSON.parse(input) }\n\
                  function success(value: string): Result<string, string> { return Ok(value) }\n\
                  function failure(message: string): Result<string, string> { return Err(message) }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].return_type,
        Type::Result(ref ok, ref error)
            if **ok == Type::JsonValue && **error == Type::String
    ));
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if matches!(
                value.kind,
                ExpressionKind::Call { ref callee, .. } if callee == "JSON.parse"
            )
    ));
    assert!(matches!(
        ir.functions[1].body[0],
        Statement::Return(ref value)
            if value.ty
                == Type::Result(Box::new(Type::String), Box::new(Type::String))
    ));
    assert!(matches!(
        ir.functions[2].body[0],
        Statement::Return(ref value)
            if value.ty
                == Type::Result(Box::new(Type::String), Box::new(Type::String))
    ));
}

#[test]
fn validates_and_lowers_safe_result_methods() {
    let source = "function parses(input: string): boolean { return JSON.parse(input).isOk() }\n\
                  function failed(value: Result<string, string>): boolean { return value.isErr() }\n\
                  function valueOr(value: Result<string, string>, fallback: string): string {\n\
                    return value.unwrapOr(fallback)\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if value.ty == Type::Bool
                && matches!(value.kind, ExpressionKind::ResultCheck { is_ok: true, .. })
    ));
    assert!(matches!(
        ir.functions[2].body[0],
        Statement::Return(ref value)
            if value.ty == Type::String
                && matches!(value.kind, ExpressionKind::ResultUnwrapOr { .. })
    ));

    let invalid = rustify_parser::parse(
        "function crossed(value: Result<string, string>): boolean { return value.isSome() }\n\
         function receiver(value: string): boolean { return value.isOk() }\n\
         function fallback(value: Result<string, string>): string { return value.unwrapOr(1) }\n\
         function arity(value: Result<string, string>): boolean { return value.isErr(1) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT054"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT034"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT030"), "{:?}", analysis.diagnostics);
}

#[test]
fn lowers_async_functions_and_await_to_typed_ir() {
    let source = "async function load(): Promise<string> { return \"ready\" }\n\
                  async function consume(): Promise<string> { return await load() }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(ir.functions[0].is_async);
    assert_eq!(ir.functions[0].return_type, Type::String);
    assert!(matches!(
        ir.functions[1].body[0],
        Statement::Return(ref value)
            if value.ty == Type::String && matches!(value.kind, ExpressionKind::Await(_))
    ));
}

#[test]
fn rejects_incoherent_async_usage() {
    let source = "async function missingPromise(): string { return \"bad\" }\n\
                  function missingAsync(): Promise<string> { return load() }\n\
                  async function invalidAwait(): Promise<string> { return await \"bad\" }\n\
                  async function load(): Promise<string> { return \"ready\" }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT042"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT043"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT045"), "{:?}", analysis.diagnostics);
}

#[test]
fn rejects_promises_in_non_representable_rust_positions() {
    let source = "type TaskBox = { task: Promise<string> }\n\
                  async function load(): Promise<string> { return \"ready\" }\n\
                  async function stored(): Promise<string> {\n\
                    const pending: Promise<string> = load()\n\
                    return await pending\n\
                  }\n\
                  async function nestedReturn(): Promise<Promise<string>> { return load() }\n\
                  function nestedParameter(values: Promise<string>[]): void { console.log() }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT060")
            .count()
            >= 4,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn rejects_ignored_promises_that_would_not_execute_as_rust_futures() {
    let source = "async function load(): Promise<string> { return \"ready\" }\n\
                  async function discard(): Promise<void> { load(); return }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT061"),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn rejects_promise_parameters_not_consumed_exactly_once() {
    let source = "async function unused(task: Promise<string>): Promise<void> { return }\n\
                  async function repeated(task: Promise<string>): Promise<string> {\n\
                    await task\n\
                    return await task\n\
                  }\n\
                  async function once(task: Promise<string>): Promise<string> { return await task }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert_eq!(
        analysis
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == "SFT062")
            .count(),
        2,
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn rejects_top_level_code_that_native_codegen_would_drop() {
    let program = rustify_parser::parse("console.log(\"must run\")\n").unwrap();
    let analysis = analyze(&program);
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT046"),
        "{:?}",
        analysis.diagnostics
    );
}

#[test]
fn lowers_typed_object_literals_with_optional_and_nested_fields() {
    let source = "type Address = { city: string }\n\
                  type User = { name: string; nickname?: string; address: Address }\n\
                  function build(): User {\n\
                    return { name: \"Ada\", address: { city: \"London\" } }\n\
                  }\n\
                  function hasNickname(user: User): boolean { return user.nickname.isSome() }\n\
                  function nicknameOr(user: User, fallback: string): string {\n\
                    return user.nickname.unwrapOr(fallback)\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if matches!(value.kind, ExpressionKind::StructLiteral { ref name, .. } if name == "User")
    ));
    assert!(matches!(
        ir.functions[1].body[0],
        Statement::Return(ref value)
            if matches!(value.kind, ExpressionKind::OptionCheck { .. })
    ));
}

#[test]
fn rejects_invalid_and_duplicate_object_literal_fields() {
    let source = "type User = { name: string }\n\
                  function invalid(): User { return { missing: \"Ada\" } }\n\
                  function duplicate(): User { return { name: \"A\", name: \"B\" } }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT047"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT049"), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_logical_unary_remainder_and_string_operations() {
    let source = "function enabled(left: boolean, right: boolean): boolean { return !left || right }\n\
                  function remainder(value: number): number { return value % 2 }\n\
                  function join(left: string, right: string): string { return left + right }\n\
                  function grouped(a: number, b: number, c: number): number { return (a + b) * c }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_and_lowers_conditional_expressions() {
    let source = "function label(enabled: boolean): string { return enabled ? \"yes\" : \"no\" }\n\
                  function maybe(enabled: boolean): string | null { return enabled ? \"yes\" : null }\n\
                  function none(enabled: boolean): string | null { return enabled ? null : null }\n\
                  function nested(first: boolean, second: boolean): string {\n\
                    return first ? (second ? \"both\" : \"first\") : \"none\"\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if value.ty == Type::String
                && matches!(value.kind, ExpressionKind::Conditional { .. })
    ));
    assert!(matches!(
        ir.functions[1].body[0],
        Statement::Return(ref value)
            if value.ty == Type::Option(Box::new(Type::String))
                && matches!(
                    value.kind,
                    ExpressionKind::Conditional {
                        ref then_value,
                        ref else_value,
                        ..
                    } if matches!(then_value.kind, ExpressionKind::Some(_))
                        && matches!(else_value.kind, ExpressionKind::Null)
                )
    ));
    assert!(matches!(
        ir.functions[2].body[0],
        Statement::Return(ref value) if value.ty == Type::Option(Box::new(Type::String))
    ));
}

#[test]
fn validates_native_math_apis() {
    let source = "function magnitude(value: number): number { return Math.abs(value) }\n\
                  function bounded(value: number, limit: number): number {\n\
                    return Math.min(Math.max(value, 0), limit)\n\
                  }\n\
                  function square(value: number): number { return Math.pow(value, 2) }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);

    let invalid = rustify_parser::parse(
        "function bad(value: string): number { return Math.abs(value) }\n\
         function missing(value: number): number { return Math.min(value) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT034"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT030"), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_collection_search_methods() {
    let source = "function contains(values: string[], target: string): boolean {\n\
                    return values.includes(target)\n\
                  }\n\
                  function matches(value: string, target: string): boolean {\n\
                    return value.includes(target) && value.startsWith(target) || value.endsWith(target)\n\
                  }\n\
                  function normalize(value: string): string {\n\
                    return value.trim().toLowerCase()\n\
                  }\n\
                  function joined(values: string[], separator: string): string {\n\
                    return values.join(separator)\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value) if matches!(value.kind, ExpressionKind::ArrayIncludes { .. })
    ));

    let invalid = rustify_parser::parse(
        "function badType(values: string[]): boolean { return values.includes(1) }\n\
         function badReceiver(values: number[]): boolean { return values.startsWith(\"x\") }\n\
         function badArity(value: string): boolean { return value.includes() }\n\
         function badTransform(value: number): string { return value.trim() }\n\
         function badTransformArity(value: string): string { return value.toUpperCase(\"x\") }\n\
         function badJoin(values: number[]): string { return values.join(\",\") }\n\
         function badJoinSeparator(values: string[]): string { return values.join(1) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT034"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT054"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT030"), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_mutable_array_push() {
    let source = "function append(values: string[], target: string): string[] {\n\
                    let result: string[] = values\n\
                    result.push(target)\n\
                    return result\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[1],
        Statement::Expression(ref value)
            if value.ty == Type::Unit && matches!(value.kind, ExpressionKind::ArrayPush { .. })
    ));

    let invalid = rustify_parser::parse(
        "type Box = { values: string[] }\n\
         function immutable(target: string): void { const values: string[] = []; values.push(target) }\n\
         function parameter(values: string[], target: string): void { values.push(target) }\n\
         function property(box: Box, target: string): void { box.values.push(target) }\n\
         function wrong(target: number): void { let values: string[] = []; values.push(target) }\n\
         function pushReturn(): void { let values: string[] = []; return values.push(\"x\") }\n\
         function pushValue(): void { let values: string[] = []; const count = values.push(\"x\") }\n\
         function nestedPush(values: string[]): void { console.log(values.push(\"x\")) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(
        codes.iter().filter(|code| **code == "SFT055").count() >= 3,
        "{:?}",
        analysis.diagnostics
    );
    assert!(codes.contains(&"SFT056"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT034"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT057"), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_mutable_array_pop() {
    let source = "function takeLast(values: string[]): string | null {\n\
                    let result: string[] = values\n\
                    return result.pop()\n\
                  }\n\
                  function discardLast(values: string[]): string[] {\n\
                    let result: string[] = values\n\
                    result.pop()\n\
                    return result\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[1],
        Statement::Return(ref value)
            if value.ty == Type::Option(Box::new(Type::String))
                && matches!(value.kind, ExpressionKind::ArrayPop(_))
    ));

    let invalid = rustify_parser::parse(
        "type Box = { values: string[] }\n\
         function immutable(): string | null { const values: string[] = []; return values.pop() }\n\
         function parameter(values: string[]): string | null { return values.pop() }\n\
         function property(box: Box): string | null { return box.values.pop() }\n\
         function badReceiver(value: string): string | null { return value.pop() }\n\
         function badArity(): string | null { let values: string[] = []; return values.pop(1) }\n\
         function nestedPop(values: string[]): void { console.log(values.pop()) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(
        codes.iter().filter(|code| **code == "SFT055").count() >= 3,
        "{:?}",
        analysis.diagnostics
    );
    assert!(codes.contains(&"SFT056"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT054"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT030"), "{:?}", analysis.diagnostics);
}

#[test]
fn lowers_safe_array_index_reads() {
    let source = "function first(values: string[]): string | null { return values[0] }\n\
                  function at(values: number[], index: number): number | null { return values[index] }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if value.ty == Type::Option(Box::new(Type::String))
                && matches!(value.kind, ExpressionKind::ArrayGet { .. })
    ));

    let invalid = rustify_parser::parse(
        "function badIndex(values: string[]): string | null { return values[\"first\"] }\n\
         function badReceiver(value: string): string | null { return value[0] }\n\
         function assignment(values: string[], index: number): void { values[index] = \"changed\" }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(
        codes.iter().filter(|code| **code == "SFT058").count() >= 2,
        "{:?}",
        analysis.diagnostics
    );
    assert!(codes.contains(&"SFT018"), "{:?}", analysis.diagnostics);
}

#[test]
fn validates_and_lowers_safe_optional_methods() {
    let source = "function hasFirst(values: string[]): boolean { return values[0].isSome() }\n\
                  function missing(value: string | null): boolean { return value.isNone() }\n\
                  function firstOr(values: string[], fallback: string): string {\n\
                    return values[0].unwrapOr(fallback)\n\
                  }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if value.ty == Type::Bool
                && matches!(value.kind, ExpressionKind::OptionCheck { is_some: true, .. })
    ));
    assert!(matches!(
        ir.functions[2].body[0],
        Statement::Return(ref value)
            if value.ty == Type::String
                && matches!(value.kind, ExpressionKind::OptionUnwrapOr { .. })
    ));

    let invalid = rustify_parser::parse(
        "function receiver(value: string): boolean { return value.isSome() }\n\
         function fallback(value: string | null): string { return value.unwrapOr(1) }\n\
         function arity(value: string | null): boolean { return value.isNone(1) }",
    )
    .unwrap();
    let analysis = analyze(&invalid);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT054"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT034"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT030"), "{:?}", analysis.diagnostics);
}

#[test]
fn rejects_invalid_conditional_expressions() {
    let source = "function badCondition(value: number): string { return value ? \"yes\" : \"no\" }\n\
                  function badBranches(enabled: boolean): string { return enabled ? \"yes\" : 1 }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT052"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT053"), "{:?}", analysis.diagnostics);
}

#[test]
fn rejects_operators_for_incompatible_operand_types() {
    let source = "function invalid(left: boolean, right: boolean): boolean { return left + right }\n\
                  function alsoInvalid(value: string): string { return -value }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    let codes: Vec<_> = analysis
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code)
        .collect();
    assert!(codes.contains(&"SFT036"), "{:?}", analysis.diagnostics);
    assert!(codes.contains(&"SFT050"), "{:?}", analysis.diagnostics);
}

#[test]
fn contextualizes_empty_array_literals() {
    let source = "function empty(): string[] { return [] }\n\
                  function local(): string[] { const values: string[] = []; return values }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Return(ref value)
            if value.ty == Type::Vec(Box::new(Type::String))
    ));
}

#[test]
fn resolves_nested_struct_properties_and_array_length() {
    let source = "type Address = { city: string }\n\
                  type User = { address: Address; tags: string[] }\n\
                  function city(user: User): string { return user.address.city }\n\
                  function tagCount(user: User): number { return user.tags.length }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
}

#[test]
fn lowers_empty_return_only_for_void_functions() {
    let valid = rustify_parser::parse("function stop(): void { return }").unwrap();
    let analysis = analyze(&valid);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    assert!(matches!(
        analysis.ir.unwrap().functions[0].body[0],
        Statement::ReturnVoid
    ));

    let invalid = rustify_parser::parse("function missing(): string { return }").unwrap();
    assert!(
        analyze(&invalid)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT033")
    );
}

#[test]
fn lowers_async_runtime_sleep_as_promise_void() {
    let source = "async function pause(milliseconds: number): Promise<void> { await Rustify.sleep(milliseconds) }";
    let program = rustify_parser::parse(source).unwrap();
    let analysis = analyze(&program);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let ir = analysis.ir.unwrap();
    assert!(matches!(
        ir.functions[0].body[0],
        Statement::Expression(ref value)
            if value.ty == Type::Unit && matches!(value.kind, ExpressionKind::Await(_))
    ));
}

#[test]
fn validates_module_bodies_against_only_visible_imports() {
    let module = rustify_parser::parse(
        "function hidden(): string { return \"hidden\" }\n\
         export function publicValue(): string { return hidden() }\n",
    )
    .unwrap();
    let importer = rustify_parser::parse(
        "import { publicValue } from \"./module\"\n\
         export function run(): string { return hidden() }\n",
    )
    .unwrap();
    let mut visible = rustify_parser::parse("").unwrap();
    visible.functions.push(module.functions[1].clone());

    assert!(validate_module_scope(&module, &visible).is_empty());
    let diagnostics = validate_module_scope(&importer, &visible);
    assert!(
        diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SFT031"),
        "{diagnostics:?}"
    );
}
