use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use rustify_analyzer::{analyze, line_column};
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
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Check {
        file: PathBuf,
        #[arg(long)]
        json: bool,
    },
    Compile {
        file: PathBuf,
        #[arg(short, long, default_value = "dist-rust")]
        out: PathBuf,
        #[arg(long)]
        cargo: bool,
    },
    Explain {
        file: PathBuf,
    },
    Init {
        #[arg(default_value = ".")]
        directory: PathBuf,
    },
}

fn main() -> Result<()> {
    match Cli::parse().command {
        Commands::Check { file, json } => check(&file, json),
        Commands::Compile { file, out, cargo } => compile(&file, &out, cargo),
        Commands::Explain { file } => explain(&file),
        Commands::Init { directory } => init(&directory),
    }
}

fn load(file: &Path) -> Result<(String, rustify_parser::Program, rustify_analyzer::Analysis)> {
    let source =
        fs::read_to_string(file).with_context(|| format!("could not read {}", file.display()))?;
    let program = rustify_parser::parse(&source)
        .with_context(|| format!("could not parse {}", file.display()))?;
    let analysis = analyze(&program);
    Ok((source, program, analysis))
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

fn compile(file: &Path, out: &Path, cargo_project: bool) -> Result<()> {
    let (source, _, analysis) = load(file)?;
    if !analysis.is_valid() {
        print_diagnostics(file, &source, &analysis.diagnostics);
        bail!("cannot compile invalid Rustify source");
    }
    let rust = rustify_codegen_rust::emit(analysis.ir.as_ref().expect("valid analysis has IR"))?;
    fs::create_dir_all(out)?;
    let target = if cargo_project {
        let src = out.join("src");
        fs::create_dir_all(&src)?;
        fs::write(
            out.join("Cargo.toml"),
            "[workspace]\n\n[package]\nname = \"rustify-output\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )?;
        src.join("lib.rs")
    } else {
        out.join(file.file_stem().unwrap_or_default())
            .with_extension("rs")
    };
    fs::write(&target, rust)?;
    let _ = Command::new("rustfmt").arg(&target).status();
    println!("Generated {}", target.display());
    Ok(())
}

fn explain(file: &Path) -> Result<()> {
    let (source, _, analysis) = load(file)?;
    if !analysis.is_valid() {
        print_diagnostics(file, &source, &analysis.diagnostics);
        bail!("cannot explain invalid Rustify source");
    }
    let ir = analysis.ir.as_ref().expect("valid analysis has IR");
    println!("Rustify translation plan for {}:", file.display());
    for structure in &ir.structs {
        println!("  type {} -> Rust struct", structure.name);
    }
    for enumeration in &ir.enums {
        println!("  enum {} -> Rust enum", enumeration.name);
    }
    for function in &ir.functions {
        println!("  function {} -> pub fn {}", function.name, function.name);
    }
    println!("\n{}", rustify_codegen_rust::emit(ir)?);
    Ok(())
}

fn init(directory: &Path) -> Result<()> {
    fs::create_dir_all(directory.join("src"))?;
    fs::write(
        directory.join("src/main.ts"),
        "type User = {\n  id: number\n  name: string\n}\n\nfunction greet(user: User): string {\n  return `Hello ${user.name}`\n}\n",
    )?;
    fs::write(
        directory.join("rustify.json"),
        "{\n  \"entry\": \"src/main.ts\",\n  \"out\": \"dist-rust\"\n}\n",
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
