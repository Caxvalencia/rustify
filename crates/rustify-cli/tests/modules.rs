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
