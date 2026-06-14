use std::path::PathBuf;
use std::process::Command;

fn temporary_project(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("rustify-{name}-{}", std::process::id()))
}

#[test]
fn init_creates_a_usable_project_config() {
    let directory = temporary_project("init");
    let _ = std::fs::remove_dir_all(&directory);
    let rustify = env!("CARGO_BIN_EXE_rustify");

    let init = Command::new(rustify)
        .args(["init", directory.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        init.status.success(),
        "{}",
        String::from_utf8_lossy(&init.stderr)
    );
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(directory.join("rustify.json")).unwrap())
            .unwrap();
    assert_eq!(config["entry"], "src/main.ts");
    assert_eq!(config["out"], "dist-rust");
    assert_eq!(config["cargo"], true);

    let nested = directory.join("src/nested");
    std::fs::create_dir_all(&nested).unwrap();
    let check = Command::new(rustify)
        .arg("check")
        .current_dir(&nested)
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );
    let compile = Command::new(rustify)
        .arg("compile")
        .current_dir(&nested)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(directory.join("dist-rust/Cargo.toml").is_file());
    assert!(directory.join("dist-rust/src/lib.rs").is_file());

    let second_init = Command::new(rustify)
        .args(["init", directory.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!second_init.status.success());
    assert!(String::from_utf8_lossy(&second_init.stderr).contains("refusing to overwrite"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn explicit_config_and_cli_output_override_project_defaults() {
    let directory = temporary_project("config");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("source")).unwrap();
    std::fs::write(
        directory.join("source/entry.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("custom.json"),
        r#"{
  "entry": "source/entry.ts",
  "out": "configured-output",
  "cargo": true,
  "package_name": "configured-package"
}"#,
    )
    .unwrap();
    let override_output = directory.join("override-output");
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args([
            "--config",
            directory.join("custom.json").to_str().unwrap(),
            "compile",
            "--out",
            override_output.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let manifest = std::fs::read_to_string(override_output.join("Cargo.toml")).unwrap();
    assert!(manifest.contains("name = \"configured-package\""));
    assert!(!directory.join("configured-output").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn explicit_entry_discovers_its_project_config() {
    let directory = temporary_project("entry-config");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{
  "entry": "src/main.ts",
  "out": "from-entry-config",
  "cargo": true,
  "package_name": "entry-configured"
}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["compile", directory.join("src/main.ts").to_str().unwrap()])
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(directory.join("from-entry-config/Cargo.toml").is_file());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn no_cargo_overrides_project_configuration() {
    let directory = temporary_project("no-cargo");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{
  "entry": "src/main.ts",
  "out": "plain-output",
  "cargo": true,
  "package_name": "configured"
}"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["compile", "--no-cargo"])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(directory.join("plain-output/main.rs").is_file());
    assert!(!directory.join("plain-output/Cargo.toml").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn generated_json_project_vendors_the_runtime() {
    let directory = temporary_project("runtime");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export function parse(input: string): Result<JsonValue, string> { return JSON.parse(input) }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "json-project" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let manifest = std::fs::read_to_string(directory.join("dist/Cargo.toml")).unwrap();
    assert!(manifest.contains("path = \"rustify-runtime\""));
    assert!(!manifest.contains(env!("CARGO_MANIFEST_DIR")));
    assert!(directory.join("dist/rustify-runtime/src/lib.rs").is_file());
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(directory.join("dist/Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        cargo.status.success(),
        "{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn generated_async_project_formats_and_compiles() {
    let directory = temporary_project("async");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export async function load(): Promise<string> { return \"ready\" }\n\
         export async function relay(): Promise<string> { return await load() }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "async-project" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&compile.stderr).contains("Rust 2015"),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(directory.join("dist/Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        cargo.status.success(),
        "{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn generated_async_runtime_project_vendors_and_compiles_runtime() {
    let directory = temporary_project("async-runtime");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export async function pause(ms: number): Promise<void> { await Rustify.sleep(ms) }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "async-runtime-project" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let runtime_manifest =
        std::fs::read_to_string(directory.join("dist/rustify-runtime/Cargo.toml")).unwrap();
    assert!(runtime_manifest.contains("futures-timer"));
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(directory.join("dist/Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        cargo.status.success(),
        "{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_mode_emits_and_runs_v8_fallback_for_dynamic_typescript() {
    let directory = temporary_project("hybrid-fallback");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "const message: any = \"hybrid fallback works\"\nconsole.log(message)\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-project", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("dist/rustify-hybrid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["target"], "javascript-fallback");
    assert_eq!(manifest["engine"], "external-v8-node");
    assert_eq!(manifest["diagnostics"][0]["code"], "SFT001");
    assert!(directory.join("dist/fallback/src/main.ts").is_file());
    let run = Command::new("npm")
        .args(["run", "--silent", "start"])
        .current_dir(directory.join("dist"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "hybrid fallback works"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_mode_falls_back_for_top_level_behavior_native_would_drop() {
    let directory = temporary_project("hybrid-top-level");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "console.log(\"top level fallback works\")\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-top-level", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(compile.status.success());
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("dist/rustify-hybrid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["diagnostics"][0]["code"], "SFT046");
    let run = Command::new("npm")
        .args(["run", "--silent", "start"])
        .current_dir(directory.join("dist"))
        .output()
        .unwrap();
    assert!(run.status.success());
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "top level fallback works"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_fallback_preserves_and_runs_relative_module_graph() {
    let directory = temporary_project("hybrid-modules");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/message.ts"),
        "export function message(): string { return \"module fallback works\" }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "import { message } from \"./message.ts\"\nconst value: any = message()\nconsole.log(value)\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-modules", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    assert!(directory.join("dist/fallback/src/main.ts").is_file());
    assert!(directory.join("dist/fallback/src/message.ts").is_file());
    let run = Command::new("npm")
        .args(["run", "--silent", "start"])
        .current_dir(directory.join("dist"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "module fallback works"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_v8_fallback_transforms_typescript_namespaces() {
    let directory = temporary_project("hybrid-transform");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "namespace Values { export const message = \"namespace fallback works\" }\nconsole.log(Values.message)\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-transform", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let run = Command::new("npm")
        .args(["run", "--silent", "start"])
        .current_dir(directory.join("dist"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "namespace fallback works"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_fallback_handles_typescript_outside_normalized_parser_subset() {
    let directory = temporary_project("hybrid-parser-fallback");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/message.ts"),
        "export const suffix = \" works\"\n\
         export default function message(): string { return \"parser fallback\" + suffix }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "import message, { suffix } from \"./message.ts\"\nconsole.log(message())\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-parser", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("dist/rustify-hybrid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["target"], "javascript-fallback");
    assert!(manifest["compiler_error"].is_string());
    assert!(directory.join("dist/fallback/src/message.ts").is_file());
    let run = Command::new("npm")
        .args(["run", "--silent", "start"])
        .current_dir(directory.join("dist"))
        .output()
        .unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "parser fallback works"
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_mode_rejects_invalid_typescript_syntax() {
    let directory = temporary_project("hybrid-invalid-syntax");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(directory.join("src/main.ts"), "function broken( {\n").unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-invalid", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(!compile.status.success());
    assert!(!directory.join("dist/rustify-hybrid.json").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn explain_reports_hybrid_fallback_decision() {
    let directory = temporary_project("hybrid-explain");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(directory.join("src/main.ts"), "const value: any = 1\n").unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-explain", "mode": "hybrid" }"#,
    )
    .unwrap();
    let explain = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["explain", "--json"])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        explain.status.success(),
        "{}",
        String::from_utf8_lossy(&explain.stderr)
    );
    let decision: serde_json::Value = serde_json::from_slice(&explain.stdout).unwrap();
    assert_eq!(decision["target"], "javascript-fallback");
    assert_eq!(decision["engine"], "external-v8-node");
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn explain_describes_native_rust_operations() {
    let directory = temporary_project("native-explain");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "function firstName(names: string[]): string | null { return names[0] }\n",
    )
    .unwrap();
    let explain = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["explain", "src/main.ts"])
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        explain.status.success(),
        "{}",
        String::from_utf8_lossy(&explain.stderr)
    );
    let stdout = String::from_utf8_lossy(&explain.stdout);
    assert!(
        stdout.contains(
            "function firstName -> pub fn first_name(names: Vec<String>) -> Option<String>"
        )
    );
    assert!(
        stdout.contains("array[index] -> checked Vec::get(index).cloned() returning Option<T>")
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_mode_records_native_compilation_when_source_is_compatible() {
    let directory = temporary_project("hybrid-native");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-native", "mode": "hybrid" }"#,
    )
    .unwrap();
    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let manifest: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(directory.join("dist/rustify-hybrid.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(manifest["target"], "native-rust");
    assert!(directory.join("dist/Cargo.toml").is_file());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn hybrid_rebuild_removes_artifacts_from_previous_target() {
    let directory = temporary_project("hybrid-rebuild");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(directory.join("src")).unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "src/main.ts", "out": "dist", "cargo": true, "package_name": "hybrid-rebuild", "mode": "hybrid" }"#,
    )
    .unwrap();
    std::fs::write(
        directory.join("src/main.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    let rustify = env!("CARGO_BIN_EXE_rustify");
    let native = Command::new(rustify)
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(native.status.success());
    assert!(directory.join("dist/Cargo.toml").is_file());

    std::fs::write(directory.join("src/main.ts"), "console.log(\"fallback\")\n").unwrap();
    let fallback = Command::new(rustify)
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(fallback.status.success());
    assert!(!directory.join("dist/Cargo.toml").exists());
    assert!(directory.join("dist/fallback/src/main.ts").is_file());

    std::fs::write(
        directory.join("src/main.ts"),
        "export function answer(): number { return 42 }\n",
    )
    .unwrap();
    let native_again = Command::new(rustify)
        .arg("compile")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(native_again.status.success());
    assert!(directory.join("dist/Cargo.toml").is_file());
    assert!(!directory.join("dist/fallback").exists());
    assert!(!directory.join("dist/package.json").exists());
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn invalid_project_config_is_rejected() {
    let directory = temporary_project("invalid-config");
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("rustify.json"),
        r#"{ "entry": "main.ts", "unexpected": true }"#,
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .arg("check")
        .current_dir(&directory)
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid Rustify config"));
    let _ = std::fs::remove_dir_all(directory);
}
