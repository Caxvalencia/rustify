use anyhow::{Context, Result, anyhow, bail};
use clap::{Parser, Subcommand, ValueEnum};
use rustify_analyzer::{analyze, line_column};
use rustify_parser::Program;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Parser)]
#[command(
    name = "rustify",
    version,
    about = "Compile strict TypeScript to readable Rust"
)]
struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Check {
        file: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
    Compile {
        file: Option<PathBuf>,
        #[arg(short, long)]
        out: Option<PathBuf>,
        #[arg(long)]
        cargo: bool,
        #[arg(long, conflicts_with = "cargo")]
        no_cargo: bool,
        #[arg(long, value_enum)]
        mode: Option<CompilationMode>,
    },
    Explain {
        file: Option<PathBuf>,
        #[arg(long)]
        json: bool,
        #[arg(long, value_enum)]
        mode: Option<CompilationMode>,
    },
    Init {
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct ProjectConfig {
    entry: PathBuf,
    out: PathBuf,
    cargo: bool,
    package_name: String,
    mode: CompilationMode,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "lowercase")]
enum CompilationMode {
    #[default]
    Native,
    Hybrid,
}

#[derive(Debug, Serialize)]
struct HybridManifest<'a> {
    version: u8,
    target: &'a str,
    engine: Option<&'a str>,
    entry: String,
    diagnostics: &'a [rustify_analyzer::Diagnostic],
    #[serde(skip_serializing_if = "Option::is_none")]
    compiler_error: Option<&'a str>,
}

impl Default for ProjectConfig {
    fn default() -> Self {
        Self {
            entry: PathBuf::from("src/main.ts"),
            out: PathBuf::from("dist-rust"),
            cargo: true,
            package_name: "rustify-output".to_owned(),
            mode: CompilationMode::Native,
        }
    }
}

struct Project {
    config: ProjectConfig,
    root: PathBuf,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check { file, json } => {
            let (file, _) = resolve_entry(file, cli.config.as_deref())?;
            check(&file, json)
        }
        Commands::Compile {
            file,
            out,
            cargo,
            no_cargo,
            mode,
        } => {
            let (file, project) = resolve_entry(file, cli.config.as_deref())?;
            let out = out
                .map(|path| absolute_from_current(&path))
                .unwrap_or_else(|| project.root.join(&project.config.out));
            compile(
                &file,
                &out,
                if no_cargo {
                    false
                } else {
                    cargo || project.config.cargo
                },
                &project.config.package_name,
                mode.unwrap_or(project.config.mode),
                &project.root,
            )
        }
        Commands::Explain { file, json, mode } => {
            let (file, project) = resolve_entry(file, cli.config.as_deref())?;
            explain(&file, json, mode.unwrap_or(project.config.mode))
        }
        Commands::Init { directory } => init(&directory),
    }
}

fn resolve_entry(file: Option<PathBuf>, config_path: Option<&Path>) -> Result<(PathBuf, Project)> {
    let explicit_entry = file.map(|path| absolute_from_current(&path));
    let project = load_project(config_path, explicit_entry.as_deref())?;
    let entry = explicit_entry.unwrap_or_else(|| project.root.join(&project.config.entry));
    Ok((entry, project))
}

fn load_project(config_path: Option<&Path>, entry_hint: Option<&Path>) -> Result<Project> {
    let path = match config_path {
        Some(path) => absolute_from_current(path),
        None => {
            let start = entry_hint
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or(std::env::current_dir()?);
            find_config(&start).unwrap_or_else(|| start.join("rustify.json"))
        }
    };
    if !path.is_file() {
        if config_path.is_some() {
            bail!("could not find Rustify config {}", path.display());
        }
        return Ok(Project {
            config: ProjectConfig {
                cargo: false,
                ..ProjectConfig::default()
            },
            root: entry_hint
                .and_then(Path::parent)
                .map(Path::to_path_buf)
                .unwrap_or(std::env::current_dir()?),
        });
    }
    let source =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let config = serde_json::from_str(&source)
        .with_context(|| format!("invalid Rustify config {}", path.display()))?;
    Ok(Project {
        config,
        root: path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf(),
    })
}

fn find_config(start: &Path) -> Option<PathBuf> {
    start
        .ancestors()
        .map(|directory| directory.join("rustify.json"))
        .find(|candidate| candidate.is_file())
}

fn absolute_from_current(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(path)
    }
}

fn load(file: &Path) -> Result<(String, Program, rustify_analyzer::Analysis)> {
    let mut visited = HashSet::new();
    let mut modules = HashMap::new();
    load_module(file, &mut visited, &mut modules)?;
    validate_imports(&modules)?;

    let mut merged = Program {
        source: String::new(),
        unsupported_top_level: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        structs: Vec::new(),
        enums: Vec::new(),
        functions: Vec::new(),
    };
    let mut paths: Vec<_> = modules.keys().collect();
    paths.sort();
    for path in paths {
        append_program(
            &mut merged,
            modules.get(path).expect("known module").clone(),
        );
    }
    let source = merged.source.clone();
    let analysis = analyze(&merged);
    Ok((source, merged, analysis))
}

fn load_module(
    file: &Path,
    visited: &mut HashSet<PathBuf>,
    modules: &mut HashMap<PathBuf, Program>,
) -> Result<()> {
    let path = file
        .canonicalize()
        .with_context(|| format!("could not resolve module {}", file.display()))?;
    if !visited.insert(path.clone()) {
        return Ok(());
    }
    let source =
        fs::read_to_string(&path).with_context(|| format!("could not read {}", path.display()))?;
    let program = rustify_parser::parse(&source)
        .with_context(|| format!("could not parse {}", path.display()))?;
    for import in &program.imports {
        if !import.source.starts_with('.') {
            continue;
        }
        let imported = resolve_import(&path, &import.source)?;
        load_module(&imported, visited, modules)?;
    }
    modules.insert(path, program);
    Ok(())
}

fn resolve_import(importer: &Path, specifier: &str) -> Result<PathBuf> {
    if !specifier.starts_with('.') {
        bail!("unsupported non-relative import `{specifier}`");
    }
    let base = importer.parent().unwrap_or_else(|| Path::new("."));
    let path = base.join(specifier);
    let candidates = if path.extension().is_some() {
        vec![path]
    } else {
        vec![path.with_extension("ts"), path.join("index.ts")]
    };
    candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .ok_or_else(|| {
            anyhow!(
                "could not resolve import `{specifier}` from {}",
                importer.display()
            )
        })
}

fn validate_imports(modules: &HashMap<PathBuf, Program>) -> Result<()> {
    for (path, program) in modules {
        for import in &program.imports {
            if !import.source.starts_with('.') {
                continue;
            }
            let target = resolve_import(path, &import.source)?.canonicalize()?;
            let target_program = modules
                .get(&target)
                .ok_or_else(|| anyhow!("module {} was not loaded", target.display()))?;
            let exports: HashSet<_> = target_program.exports.iter().map(String::as_str).collect();
            for name in &import.names {
                if !exports.contains(name.as_str()) {
                    bail!("module `{}` does not export `{name}`", import.source);
                }
            }
        }
    }
    Ok(())
}

fn append_program(target: &mut Program, mut module: Program) {
    let offset = target.source.len();
    shift_spans(&mut module, offset);
    target.source.push_str(&module.source);
    target.source.push('\n');
    target
        .unsupported_top_level
        .extend(module.unsupported_top_level);
    target.imports.extend(module.imports);
    target.exports.extend(module.exports);
    target.structs.extend(module.structs);
    target.enums.extend(module.enums);
    target.functions.extend(module.functions);
}

fn shift_spans(program: &mut Program, offset: usize) {
    for span in program
        .imports
        .iter_mut()
        .map(|item| &mut item.span)
        .chain(program.unsupported_top_level.iter_mut())
        .chain(program.structs.iter_mut().map(|item| &mut item.span))
        .chain(program.enums.iter_mut().map(|item| &mut item.span))
        .chain(program.functions.iter_mut().map(|item| &mut item.span))
    {
        span.start += offset;
        span.end += offset;
    }
}

fn check(file: &Path, json: bool) -> Result<()> {
    let (source, _, analysis) = load(file)?;
    if json {
        println!("{}", serde_json::to_string_pretty(&analysis.diagnostics)?);
    } else {
        print_diagnostics(file, &source, &analysis.diagnostics);
        if analysis.is_valid() {
            println!("Rustify check passed: {}", file.display());
        }
    }
    if analysis.is_valid() {
        Ok(())
    } else {
        bail!(
            "Rustify check failed with {} diagnostic(s)",
            analysis.diagnostics.len()
        )
    }
}

fn compile(
    file: &Path,
    out: &Path,
    cargo_project: bool,
    package_name: &str,
    mode: CompilationMode,
    project_root: &Path,
) -> Result<()> {
    let (source, _, analysis) = match load(file) {
        Ok(loaded) => loaded,
        Err(error)
            if matches!(mode, CompilationMode::Hybrid) && is_hybrid_eligible_error(&error) =>
        {
            let message = error.to_string();
            return write_hybrid_fallback(file, out, project_root, &[], Some(&message));
        }
        Err(error) => return Err(error),
    };
    if !analysis.is_valid() {
        if matches!(mode, CompilationMode::Hybrid) {
            return write_hybrid_fallback(file, out, project_root, &analysis.diagnostics, None);
        }
        print_diagnostics(file, &source, &analysis.diagnostics);
        bail!("cannot compile invalid Rustify source");
    }
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().expect("valid analysis has IR"))?;
    clean_fallback_artifacts(out)?;
    fs::create_dir_all(out)?;
    let target = if cargo_project {
        let src = out.join("src");
        fs::create_dir_all(&src)?;
        let ir = analysis.ir.as_ref().expect("valid analysis has IR");
        let mut manifest = format!(
            "[workspace]\n\n[package]\nname = {package_name:?}\nversion = \"0.1.0\"\nedition = \"2024\"\n"
        );
        if rustify_codegen_rust::uses_runtime(ir) {
            manifest
                .push_str("\n[dependencies]\nrustify-runtime = { path = \"rustify-runtime\" }\n");
            write_runtime_package(out)?;
        }
        fs::write(out.join("Cargo.toml"), manifest)?;
        src.join("lib.rs")
    } else {
        out.join(file.file_stem().unwrap_or_default())
            .with_extension("rs")
    };
    fs::write(&target, rust)?;
    let _ = Command::new("rustfmt")
        .args(["--edition", "2024"])
        .arg(&target)
        .status();
    if matches!(mode, CompilationMode::Hybrid) {
        write_hybrid_manifest(
            out,
            &HybridManifest {
                version: 1,
                target: "native-rust",
                engine: None,
                entry: target
                    .strip_prefix(out)
                    .unwrap_or(&target)
                    .to_string_lossy()
                    .into_owned(),
                diagnostics: &[],
                compiler_error: None,
            },
        )?;
    }
    println!("Generated {}", target.display());
    Ok(())
}

fn write_hybrid_fallback(
    file: &Path,
    out: &Path,
    project_root: &Path,
    diagnostics: &[rustify_analyzer::Diagnostic],
    compiler_error: Option<&str>,
) -> Result<()> {
    let project_root = project_root.canonicalize()?;
    let modules = collect_module_paths(file)
        .or_else(|_| collect_project_typescript_paths(&project_root, out))?;
    clean_native_artifacts(file, out)?;
    let fallback = out.join("fallback");
    fs::create_dir_all(&fallback)?;
    for module in modules {
        let relative = module.strip_prefix(&project_root).with_context(|| {
            format!(
                "hybrid module {} is outside project root {}",
                module.display(),
                project_root.display()
            )
        })?;
        let target = fallback.join(relative);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&module, target)?;
    }
    let entry = file
        .canonicalize()?
        .strip_prefix(&project_root)
        .context("hybrid entry is outside the project root")?
        .to_string_lossy()
        .into_owned();
    write_hybrid_manifest(
        out,
        &HybridManifest {
            version: 1,
            target: "javascript-fallback",
            engine: Some("external-v8-node"),
            entry: format!("fallback/{entry}"),
            diagnostics,
            compiler_error,
        },
    )?;
    let command = format!("node --no-warnings --experimental-transform-types \"fallback/{entry}\"");
    let package = serde_json::json!({
        "private": true,
        "type": "module",
        "engines": { "node": ">=22.7.0" },
        "scripts": { "start": command }
    });
    fs::write(
        out.join("package.json"),
        format!("{}\n", serde_json::to_string_pretty(&package)?),
    )?;
    print_diagnostics(file, &fs::read_to_string(file)?, diagnostics);
    println!(
        "Generated hybrid JavaScript fallback at {}",
        fallback.display()
    );
    Ok(())
}

fn write_hybrid_manifest(out: &Path, manifest: &HybridManifest<'_>) -> Result<()> {
    fs::create_dir_all(out)?;
    fs::write(
        out.join("rustify-hybrid.json"),
        format!("{}\n", serde_json::to_string_pretty(manifest)?),
    )?;
    Ok(())
}

fn clean_fallback_artifacts(out: &Path) -> Result<()> {
    if previous_hybrid_target(out).as_deref() != Some("javascript-fallback") {
        return Ok(());
    }
    remove_generated_path(&out.join("fallback"))?;
    remove_generated_path(&out.join("package.json"))
}

fn clean_native_artifacts(file: &Path, out: &Path) -> Result<()> {
    if previous_hybrid_target(out).as_deref() != Some("native-rust") {
        return Ok(());
    }
    for path in [
        out.join("Cargo.toml"),
        out.join("Cargo.lock"),
        out.join("src/lib.rs"),
        out.join("rustify-runtime"),
        out.join("target"),
        out.join(file.file_stem().unwrap_or_default())
            .with_extension("rs"),
    ] {
        remove_generated_path(&path)?;
    }
    Ok(())
}

fn previous_hybrid_target(out: &Path) -> Option<String> {
    let manifest: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(out.join("rustify-hybrid.json")).ok()?).ok()?;
    manifest["target"].as_str().map(str::to_owned)
}

fn remove_generated_path(path: &Path) -> Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else if path.exists() {
        fs::remove_file(path)?;
    }
    Ok(())
}

fn collect_module_paths(entry: &Path) -> Result<Vec<PathBuf>> {
    fn visit(file: &Path, paths: &mut HashSet<PathBuf>) -> Result<()> {
        let path = file.canonicalize()?;
        if !paths.insert(path.clone()) {
            return Ok(());
        }
        let source = fs::read_to_string(&path)?;
        let program = rustify_parser::parse(&source)?;
        for import in &program.imports {
            if import.source.starts_with('.') {
                visit(&resolve_import(&path, &import.source)?, paths)?;
            }
        }
        Ok(())
    }

    let mut paths = HashSet::new();
    visit(entry, &mut paths)?;
    let mut paths: Vec<_> = paths.into_iter().collect();
    paths.sort();
    Ok(paths)
}

fn collect_project_typescript_paths(project_root: &Path, out: &Path) -> Result<Vec<PathBuf>> {
    fn visit(directory: &Path, out: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
        for entry in fs::read_dir(directory)? {
            let path = entry?.path();
            if path == out
                || path.file_name().is_some_and(|name| {
                    matches!(name.to_str(), Some(".git" | "node_modules" | "target"))
                })
            {
                continue;
            }
            if path.is_dir() {
                visit(&path, out, paths)?;
            } else if matches!(
                path.extension().and_then(|extension| extension.to_str()),
                Some("ts" | "tsx" | "mts" | "cts")
            ) {
                paths.push(path.canonicalize()?);
            }
        }
        Ok(())
    }

    let mut paths = Vec::new();
    visit(project_root, out, &mut paths)?;
    paths.sort();
    Ok(paths)
}

fn is_hybrid_eligible_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        matches!(
            cause.downcast_ref::<rustify_parser::ParseError>(),
            Some(rustify_parser::ParseError::Declaration(_))
        )
    })
}

fn write_runtime_package(out: &Path) -> Result<()> {
    let runtime = out.join("rustify-runtime");
    fs::create_dir_all(runtime.join("src"))?;
    fs::write(
        runtime.join("Cargo.toml"),
        "[package]\nname = \"rustify-runtime\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[dependencies]\nfutures-timer = \"3\"\nserde = { version = \"1\", features = [\"derive\"] }\nserde_json = \"1\"\n",
    )?;
    fs::write(
        runtime.join("src/lib.rs"),
        include_str!("../../rustify-runtime/src/lib.rs"),
    )?;
    Ok(())
}

fn explain(file: &Path, json: bool, mode: CompilationMode) -> Result<()> {
    let (source, _, analysis) = match load(file) {
        Ok(loaded) => loaded,
        Err(error)
            if matches!(mode, CompilationMode::Hybrid) && is_hybrid_eligible_error(&error) =>
        {
            if json {
                let message = error.to_string();
                println!(
                    "{}",
                    serde_json::to_string_pretty(&HybridManifest {
                        version: 1,
                        target: "javascript-fallback",
                        engine: Some("external-v8-node"),
                        entry: file.to_string_lossy().into_owned(),
                        diagnostics: &[],
                        compiler_error: Some(&message),
                    })?
                );
            } else {
                println!("Hybrid decision: JavaScript fallback on external V8/Node");
                println!("Reason: {error}");
            }
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    if !analysis.is_valid() {
        if matches!(mode, CompilationMode::Hybrid) {
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&HybridManifest {
                        version: 1,
                        target: "javascript-fallback",
                        engine: Some("external-v8-node"),
                        entry: file.to_string_lossy().into_owned(),
                        diagnostics: &analysis.diagnostics,
                        compiler_error: None,
                    })?
                );
            } else {
                print_diagnostics(file, &source, &analysis.diagnostics);
                println!(
                    "Hybrid decision: JavaScript fallback on external V8/Node ({} diagnostic(s))",
                    analysis.diagnostics.len()
                );
            }
            return Ok(());
        }
        print_diagnostics(file, &source, &analysis.diagnostics);
        bail!("cannot explain invalid Rustify source");
    }
    let ir = analysis.ir.as_ref().expect("valid analysis has IR");
    if json {
        println!("{}", serde_json::to_string_pretty(ir)?);
        return Ok(());
    }
    println!("Rustify translation plan for {}:", file.display());
    println!("{}", rustify_codegen_rust::explain(ir));
    println!("\n{}", rustify_codegen_rust::emit(ir)?);
    Ok(())
}

fn init(directory: &Path) -> Result<()> {
    if directory.join("rustify.json").exists() || directory.join("src/main.ts").exists() {
        bail!(
            "refusing to overwrite an existing Rustify project in {}",
            directory.display()
        );
    }
    fs::create_dir_all(directory.join("src"))?;
    fs::write(
        directory.join("src/main.ts"),
        "type User = {\n  id: number\n  name: string\n}\n\nfunction greet(user: User): string {\n  return `Hello ${user.name}`\n}\n",
    )?;
    fs::write(
        directory.join("rustify.json"),
        format!(
            "{}\n",
            serde_json::to_string_pretty(&ProjectConfig::default())?
        ),
    )?;
    println!("Initialized Rustify project in {}", directory.display());
    Ok(())
}

fn print_diagnostics(file: &Path, source: &str, diagnostics: &[rustify_analyzer::Diagnostic]) {
    for diagnostic in diagnostics {
        let (line, column) = line_column(source, diagnostic.span.start);
        eprintln!(
            "{}:{}:{}: {} [{}]",
            file.display(),
            line,
            column,
            diagnostic.message,
            diagnostic.code
        );
        if let Some(hint) = &diagnostic.hint {
            eprintln!("  Hint: {hint}");
        }
    }
}
