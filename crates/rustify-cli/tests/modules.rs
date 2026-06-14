use std::path::PathBuf;
use std::process::Command;

#[test]
fn checks_and_compiles_relative_modules() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let entry = root.join("examples/modules/main.ts");
    let output = std::env::temp_dir().join(format!("rustify-modules-{}", std::process::id()));
    let rustify = env!("CARGO_BIN_EXE_rustify");

    let check = Command::new(rustify)
        .args(["check", entry.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(
        check.status.success(),
        "{}",
        String::from_utf8_lossy(&check.stderr)
    );

    let compile = Command::new(rustify)
        .args([
            "compile",
            entry.to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
            "--cargo",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(output.join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(
        cargo.status.success(),
        "{}",
        String::from_utf8_lossy(&cargo.stderr)
    );
    let _ = std::fs::remove_dir_all(output);
}

#[test]
fn rejects_importing_private_symbols() {
    let directory =
        std::env::temp_dir().join(format!("rustify-private-module-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("private.ts"),
        "function hidden(): string { return \"hidden\" }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { hidden } from \"./private\"\nexport function run(): string { return hidden() }\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["check", directory.join("main.ts").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("does not export `hidden`"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn rejects_using_private_symbols_without_importing_them() {
    let directory =
        std::env::temp_dir().join(format!("rustify-private-leak-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("helpers.ts"),
        "function hidden(): string { return \"hidden\" }\n\
         export function publicValue(): string { return hidden() }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { publicValue } from \"./helpers\"\n\
         export function run(): string { return hidden() }\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["check", directory.join("main.ts").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown function `hidden`"));
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn compiles_duplicate_private_symbols_in_isolated_modules() {
    let directory =
        std::env::temp_dir().join(format!("rustify-private-isolation-{}", std::process::id()));
    let output = directory.join("output");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("first.ts"),
        "function hidden(): string { return \"first\" }\n\
         function unusedPrivate(): string { return \"unused\" }\n\
         export function first(): string { return hidden() }\n\
         export function unusedApi(): string { return unusedPrivate() }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("second.ts"),
        "function hidden(): string { return \"second\" }\n\
         export function second(): string { return hidden() }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { first, unusedApi } from \"./first\"\n\
         import { second } from \"./second\"\n\
         export function combined(): string { return first() + second() }\n",
    )
    .unwrap();

    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args([
            "compile",
            directory.join("main.ts").to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
            "--cargo",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let rust = std::fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert_eq!(rust.matches("fn hidden()").count(), 2, "{rust}");
    assert!(!rust.contains("pub fn hidden()"), "{rust}");
    assert!(rust.contains("pub use rustify_main::*;"), "{rust}");

    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(output.join("Cargo.toml"))
        .env("RUSTFLAGS", "-D warnings")
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
fn compiles_aliased_type_and_function_imports() {
    let directory =
        std::env::temp_dir().join(format!("rustify-import-aliases-{}", std::process::id()));
    let output = directory.join("output");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("models.ts"),
        "export type User = { name: string }\n\
         export function greet(user: User): string { return user.name }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { User as Person, greet as welcome } from \"./models\"\n\
         export function run(person: Person): string { return welcome(person) }\n",
    )
    .unwrap();

    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args([
            "compile",
            directory.join("main.ts").to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
            "--cargo",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let rust = std::fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert!(rust.contains("User as Person"), "{rust}");
    assert!(rust.contains("greet as welcome"), "{rust}");
    assert!(
        rust.contains("pub fn run(person: Person) -> String"),
        "{rust}"
    );

    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(output.join("Cargo.toml"))
        .env("RUSTFLAGS", "-D warnings")
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
fn compiles_transitive_named_reexports() {
    let directory = std::env::temp_dir().join(format!("rustify-reexports-{}", std::process::id()));
    let output = directory.join("output");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("models.ts"),
        "export type User = { name: string }\n\
         export function greet(user: User): string { return user.name }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("public.ts"),
        "export { User as Person, greet as welcome } from \"./models\"\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { Person, welcome } from \"./public\"\n\
         export function run(person: Person): string { return welcome(person) }\n",
    )
    .unwrap();

    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args([
            "compile",
            directory.join("main.ts").to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
            "--cargo",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let rust = std::fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert!(
        rust.contains("pub use super::rustify_models::{User as Person, greet as welcome};"),
        "{rust}"
    );
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(output.join("Cargo.toml"))
        .env("RUSTFLAGS", "-D warnings")
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
fn compiles_default_function_and_type_imports() {
    let directory =
        std::env::temp_dir().join(format!("rustify-default-exports-{}", std::process::id()));
    let output = directory.join("output");
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("greeter.ts"),
        "export default function greet(name: string): string { return name }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("models.ts"),
        "export default interface User { name: string }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import welcome from \"./greeter\"\n\
         import Person from \"./models\"\n\
         export function run(person: Person): string { return welcome(person.name) }\n",
    )
    .unwrap();

    let compile = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args([
            "compile",
            directory.join("main.ts").to_str().unwrap(),
            "--out",
            output.to_str().unwrap(),
            "--cargo",
        ])
        .output()
        .unwrap();
    assert!(
        compile.status.success(),
        "{}",
        String::from_utf8_lossy(&compile.stderr)
    );
    let rust = std::fs::read_to_string(output.join("src/lib.rs")).unwrap();
    assert!(rust.contains("pub use self::greet as default;"), "{rust}");
    assert!(rust.contains("pub use self::User as Default;"), "{rust}");
    assert!(rust.contains("default as welcome"), "{rust}");
    assert!(rust.contains("Default as Person"), "{rust}");
    let cargo = Command::new("cargo")
        .args(["check", "--manifest-path"])
        .arg(output.join("Cargo.toml"))
        .env("RUSTFLAGS", "-D warnings")
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
fn rejects_cyclic_module_graphs() {
    let directory =
        std::env::temp_dir().join(format!("rustify-module-cycle-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("a.ts"),
        "import { fromB } from \"./b\"\n\
         export function fromA(): string { return fromB() }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("b.ts"),
        "import { fromA } from \"./a\"\n\
         export function fromB(): string { return fromA() }\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["check", directory.join("a.ts").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("cyclic Rustify modules are not supported")
    );
    let _ = std::fs::remove_dir_all(directory);
}

#[test]
fn rejects_rust_name_collisions_across_modules() {
    let directory =
        std::env::temp_dir().join(format!("rustify-module-collision-{}", std::process::id()));
    std::fs::create_dir_all(&directory).unwrap();
    std::fs::write(
        directory.join("camel.ts"),
        "export function loadValue(): number { return 1 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("snake.ts"),
        "export function load_value(): number { return 2 }\n",
    )
    .unwrap();
    std::fs::write(
        directory.join("main.ts"),
        "import { loadValue } from \"./camel\"\n\
         import { load_value } from \"./snake\"\n\
         export function run(): number { return loadValue() + load_value() }\n",
    )
    .unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_rustify"))
        .args(["check", directory.join("main.ts").to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("SFT064"));
    let _ = std::fs::remove_dir_all(directory);
}
