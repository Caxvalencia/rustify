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
    assert!(!rust.contains("user.id.clone()"));
}

#[test]
fn generated_control_flow_compiles_with_rustc() {
    let source = "function sum(values: number[]): number {\n\
                    let total: number = 0\n\
                    for (const value of values) {\n\
                      if (value === 0) { continue }\n\
                      total = total + value\n\
                      if (total > 100) { break }\n\
                    }\n\
                    return total\n\
                  }\n\
                  function describe(value: number): string {\n\
                    if (value > 0) { return \"positive\" } else { return \"negative\" }\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("continue;"));
    assert!(rust.contains("break;"));
    let directory = std::env::temp_dir().join(format!("rustify-test-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("generated.rs");
    let output_path = directory.join("generated.rlib");
    std::fs::write(&source_path, rust).unwrap();
    let output = std::process::Command::new("rustc")
        .args(["--crate-type", "lib"])
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn generated_enums_options_and_void_compile_with_rustc() {
    let source = "enum Status { Active, Inactive }\n\
                  function current(): Status { return Status.Active }\n\
                  function maybe(enabled: boolean): string | null {\n\
                    if (enabled) { return \"yes\" }\n\
                    return null\n\
                  }\n\
                  function announce(status: Status): void { console.log(status) }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("Status::Active"));
    assert!(rust.contains("Some(\"yes\".to_string())"));
    assert!(rust.contains("return None;"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_multi_argument_console_log_compiles_with_rustc() {
    let source = "function report(label: string, value: number, active: boolean): void {\n\
                    console.log(label, value, `active=${active}`)\n\
                    console.log()\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(
        rust.contains("println!(\"{:?} {:?} {}\", label, value, format!(\"active={}\", active));")
    );
    assert!(rust.contains("println!();"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_camel_case_identifiers_compile_without_warnings() {
    let source = "type User = { firstName: string; type: string }\n\
                  function displayName(userValue: User): string { return userValue.firstName }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub first_name: String"));
    assert!(rust.contains("pub r#type: String"));
    assert!(rust.contains("pub fn display_name(user_value: User)"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_type_and_variant_identifiers_compile_without_warnings() {
    let source = "type user_record = { display_name: string }\n\
                  enum task_status { in_progress, $done }\n\
                  function build_record(): user_record {\n\
                    return { display_name: \"ready\" }\n\
                  }\n\
                  function current_status(): task_status { return task_status.in_progress }\n\
                  function done_status(): task_status { return task_status.$done }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub struct UserRecord"));
    assert!(rust.contains("pub enum TaskStatus"));
    assert!(rust.contains("TaskStatus::InProgress"));
    assert!(rust.contains("TaskStatus::DollarDone"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_non_copy_identifiers_remain_reusable() {
    let source = "function consume(value: string): void { console.log(value) }\n\
                  function reusable(value: string): string {\n\
                    consume(value)\n\
                    return value\n\
                  }\n\
                  function appendAndKeep(values: string[], target: string): string {\n\
                    let result: string[] = values\n\
                    result.push(target)\n\
                    return target\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("consume(value.clone());"));
    assert!(rust.contains("return value.clone();"));
    assert!(rust.contains("let mut result: Vec<String> = values.clone();"));
    assert!(rust.contains("result.push(target.clone());"));
    assert!(rust.contains("return target.clone();"));
    compile_with_rustc(&rust);
}

#[test]
fn detects_runtime_usage_for_json_but_not_plain_results() {
    let json = rustify_analyzer::analyze(
        &rustify_parser::parse(
            "function parse(input: string): Result<JsonValue, string> { return JSON.parse(input) }",
        )
        .unwrap(),
    );
    let plain = rustify_analyzer::analyze(
        &rustify_parser::parse(
            "function success(value: string): Result<string, string> { return Ok(value) }",
        )
        .unwrap(),
    );
    assert!(rustify_codegen_rust::uses_runtime(
        json.ir.as_ref().unwrap()
    ));
    assert!(!rustify_codegen_rust::uses_runtime(
        plain.ir.as_ref().unwrap()
    ));

    let sleep = rustify_analyzer::analyze(
        &rustify_parser::parse(
            "async function pause(ms: number): Promise<void> { await Rustify.sleep(ms) }",
        )
        .unwrap(),
    );
    assert!(rustify_codegen_rust::uses_runtime(
        sleep.ir.as_ref().unwrap()
    ));
    let rust = rustify_codegen_rust::emit(sleep.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub async fn pause(ms: f64) {"));
    assert!(rust.contains("rustify_runtime::async_runtime::sleep(ms).await"));
}

#[test]
fn generated_safe_result_methods_compile_with_rustc() {
    let source = "function succeeded(value: Result<string, string>): boolean { return value.isOk() }\n\
                  function failed(value: Result<string, string>): boolean { return value.isErr() }\n\
                  function valueOr(value: Result<string, string>, fallback: string): string {\n\
                    return value.unwrapOr(fallback)\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("value.is_ok()"));
    assert!(rust.contains("value.is_err()"));
    assert!(rust.contains("value.clone().unwrap_or(fallback.clone())"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_async_functions_and_await_compile_with_rustc() {
    let source = "async function loadMessage(): Promise<string> { return \"ready\" }\n\
                  async function relayMessage(): Promise<string> { return await loadMessage() }\n\
                  async function awaitTask(task: Promise<string>): Promise<string> {\n\
                    return await task\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub async fn load_message() -> String"));
    assert!(rust.contains("return load_message().await;"));
    assert!(rust.contains(
        "pub async fn await_task(task: impl std::future::Future<Output = String>) -> String"
    ));
    compile_with_rustc(&rust);
}

#[test]
fn generated_unused_parameters_compile_without_warnings() {
    let source = "function ignore(value: string, count: number): void { return }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let _ = &value;"));
    assert!(rust.contains("let _ = &count;"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_local_bindings_compile_without_warnings() {
    let source = "function stable(): number {\n\
                    let value: number = 1\n\
                    const unused: string = \"ignored\"\n\
                    return value\n\
                  }\n\
                  function update(): number {\n\
                    let value: number = 1\n\
                    value = 2\n\
                    return value\n\
                  }\n\
                  function ignoreItems(values: string[]): void {\n\
                    for (const value of values) { console.log() }\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let value: f64 = 1.0;"));
    assert!(rust.contains("let mut value: f64 = 1.0;"));
    assert!(rust.contains("let _ = &unused;"));
    assert!(rust.contains("let _ = &value;\n        println!();"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_shadowed_bindings_compile_without_warnings() {
    let source = "function shadowParameter(value: number): number {\n\
                    let value: number = 1\n\
                    return value\n\
                  }\n\
                  function shadowMutable(): number {\n\
                    let value: number = 1\n\
                    if (true) {\n\
                      let value: number = 2\n\
                      value = 3\n\
                    }\n\
                    return value\n\
                  }\n\
                  function shadowLoop(value: string, values: string[]): void {\n\
                    for (const value of values) { console.log(value) }\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let _ = &value;\n    let value: f64 = 1.0;"));
    assert!(rust.contains("let value: f64 = 1.0;\n    let _ = &value;\n    if true"));
    assert!(rust.contains("let mut value: f64 = 2.0;"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_unreachable_statements_are_omitted() {
    let source = "function afterReturn(): number {\n\
                    let value: number = 1\n\
                    return value\n\
                    value = 2\n\
                  }\n\
                  function afterBranches(enabled: boolean): number {\n\
                    if (enabled) { return 1 } else { return 2 }\n\
                    console.log(\"unreachable\")\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(!rust.contains("value = 2.0;"));
    assert!(!rust.contains("unreachable"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_struct_literals_compile_with_rustc() {
    let source = "type Address = { city: string }\n\
                  type User = { name: string; nickname?: string; address: Address }\n\
                  function consume(value: string): void { console.log(value) }\n\
                  function buildUser(): User {\n\
                    return { name: \"Ada\", address: { city: \"London\" } }\n\
                  }\n\
                  function reusableName(user: User): string {\n\
                    consume(user.name)\n\
                    return user.name\n\
                  }\n\
                  function nicknameOr(user: User, fallback: string): string {\n\
                    return user.nickname.unwrapOr(fallback)\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("User {"));
    assert!(rust.contains("nickname: None"));
    assert!(rust.contains("Address {"));
    assert!(rust.contains("consume(user.name.clone());"));
    assert!(rust.contains("return user.name.clone();"));
    assert!(rust.contains("user.nickname.clone().unwrap_or(fallback.clone())"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_basic_operations_compile_with_rustc() {
    let source = "function enabled(left: boolean, right: boolean): boolean { return !left || right }\n\
                  function remainder(value: number): number { return value % 2 }\n\
                  function join(left: string, right: string): string { return left + right }\n\
                  function grouped(a: number, b: number, c: number): number { return (a + b) * c }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("!(left) || right"));
    assert!(rust.contains("value % 2.0"));
    assert!(rust.contains("format!(\"{}{}\", left, right)"));
    assert!(rust.contains("(a + b) * c"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_conditional_expressions_compile_with_rustc() {
    let source = "function label(enabled: boolean): string { return enabled ? \"yes\" : \"no\" }\n\
                  function maybe(enabled: boolean): string | null { return enabled ? \"yes\" : null }\n\
                  function nested(first: boolean, second: boolean): string {\n\
                    return first ? (second ? \"both\" : \"first\") : \"none\"\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("if enabled { \"yes\".to_string() } else { \"no\".to_string() }"));
    assert!(rust.contains("if enabled { Some(\"yes\".to_string()) } else { None }"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_math_apis_compile_with_rustc() {
    let source = "function rounded(value: number): number { return Math.round(value) }\n\
                  function bounded(value: number, limit: number): number {\n\
                    return Math.min(Math.max(value, 0), limit)\n\
                  }\n\
                  function square(value: number): number { return Math.pow(value, 2) }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("f64::round(value)"));
    assert!(rust.contains("f64::min(f64::max(value, 0.0), limit)"));
    assert!(rust.contains("f64::powf(value, 2.0)"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_collection_search_methods_compile_with_rustc() {
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
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("values.contains(&target)"));
    assert!(rust.contains("value.contains(target.as_str())"));
    assert!(rust.contains("value.starts_with(target.as_str())"));
    assert!(rust.contains("value.ends_with(target.as_str())"));
    assert!(rust.contains("value.trim().to_string().to_lowercase()"));
    assert!(rust.contains("values.join(separator.as_str())"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_mutable_array_push_compiles_with_rustc() {
    let source = "function append(values: string[], target: string): string[] {\n\
                    let result: string[] = values\n\
                    result.push(target)\n\
                    return result\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let mut result: Vec<String> = values.clone();"));
    assert!(rust.contains("result.push(target.clone());"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_mutable_array_pop_compiles_with_rustc() {
    let source = "function takeLast(values: string[]): string | null {\n\
                    let result: string[] = values\n\
                    return result.pop()\n\
                  }\n\
                  function discardLast(values: string[]): string[] {\n\
                    let result: string[] = values\n\
                    result.pop()\n\
                    return result\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("return result.pop();"));
    assert!(rust.contains("let _ = result.pop();"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_safe_array_index_reads_compile_with_rustc() {
    let source = "function first(values: string[]): string | null { return values[0] }\n\
                  function at(values: number[], index: number): number | null { return values[index] }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("rustify_index_value >= 0.0"));
    assert!(rust.contains(".get(rustify_index_value as usize).cloned()"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_safe_optional_methods_compile_with_rustc() {
    let source = "function hasFirst(values: string[]): boolean { return values[0].isSome() }\n\
                  function missing(value: string | null): boolean { return value.isNone() }\n\
                  function firstOr(values: string[], fallback: string): string {\n\
                    return values[0].unwrapOr(fallback)\n\
                  }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains(".is_some()"));
    assert!(rust.contains("value.is_none()"));
    assert!(rust.contains(".unwrap_or(fallback.clone())"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_empty_arrays_compile_with_rustc() {
    let source = "function empty(): string[] { return [] }\n\
                  function local(): string[] { const values: string[] = []; return values }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let values: Vec<String> = vec![];"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_nested_properties_and_array_length_compile_with_rustc() {
    let source = "type Address = { city: string }\n\
                  type User = { address: Address; tags: string[] }\n\
                  function city(user: User): string { return user.address.city }\n\
                  function tagCount(user: User): number { return user.tags.length }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("user.address.city.clone()"));
    assert!(rust.contains("user.tags.len() as f64"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_empty_void_return_compiles_with_rustc() {
    let parsed = rustify_parser::parse("function stop(): void { return }").unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("pub fn stop() {"));
    assert!(rust.contains("return;"));
    compile_with_rustc(&rust);
}

#[test]
fn generated_ignored_must_use_values_compile_without_warnings() {
    let source = "function maybe(enabled: boolean): string | null {\n\
                    return enabled ? \"ready\" : null\n\
                  }\n\
                  function discard(enabled: boolean): void { maybe(enabled) }";
    let parsed = rustify_parser::parse(source).unwrap();
    let analysis = rustify_analyzer::analyze(&parsed);
    assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().unwrap()).unwrap();
    assert!(rust.contains("let _ = maybe(enabled);"));
    compile_with_rustc(&rust);
}

fn compile_with_rustc(rust: &str) {
    static NEXT_DIRECTORY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = NEXT_DIRECTORY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "rustify-types-test-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(&directory).unwrap();
    let source_path = directory.join("generated.rs");
    let output_path = directory.join("generated.rlib");
    std::fs::write(&source_path, rust).unwrap();
    let output = std::process::Command::new("rustc")
        .args(["--crate-type", "lib", "--edition", "2024", "-D", "warnings"])
        .arg(&source_path)
        .arg("-o")
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}
