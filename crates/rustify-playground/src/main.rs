use anyhow::Result;
use axum::{
    Json, Router,
    http::StatusCode,
    response::Html,
    routing::{get, post},
};
use rustify_analyzer::Diagnostic;
use serde::{Deserialize, Serialize};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

const DEFAULT_SOURCE: &str = r#"type User = {
  name: string
  active: boolean
}

function greet(user: User): string {
  return `Hello ${user.name}`
}

function buildUser(): User {
  return { name: "Ada", active: true }
}"#;

const INDEX_HTML: &str = include_str!("index.html");

#[derive(Debug, Deserialize)]
struct CompileRequest {
    source: String,
}

#[derive(Debug, Serialize)]
struct CompileResponse {
    valid: bool,
    rust: Option<String>,
    diagnostics: Vec<Diagnostic>,
}

fn compile_source(source: &str) -> CompileResponse {
    let program = match rustify_parser::parse(source) {
        Ok(program) => program,
        Err(error) => {
            return CompileResponse {
                valid: false,
                rust: None,
                diagnostics: vec![Diagnostic {
                    code: "PARSE",
                    severity: rustify_analyzer::Severity::Error,
                    message: error.to_string(),
                    hint: None,
                    span: rustify_parser::Span { start: 0, end: 0 },
                }],
            };
        }
    };
    let analysis = rustify_analyzer::analyze(&program);
    let rust = analysis
        .ir
        .as_ref()
        .and_then(|ir| rustify_codegen_rust::emit(ir).ok());
    CompileResponse {
        valid: analysis.is_valid() && rust.is_some(),
        rust,
        diagnostics: analysis.diagnostics,
    }
}

async fn index() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn example() -> &'static str {
    DEFAULT_SOURCE
}

async fn favicon() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn compile(Json(request): Json<CompileRequest>) -> Json<CompileResponse> {
    Json(compile_source(&request.source))
}

#[tokio::main]
async fn main() -> Result<()> {
    let port = std::env::var("RUSTIFY_PLAYGROUND_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(3000);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    let app = Router::new()
        .route("/", get(index))
        .route("/favicon.ico", get(favicon))
        .route("/api/example", get(example))
        .route("/api/compile", post(compile));
    let listener = tokio::net::TcpListener::bind(address).await?;
    println!("Rustify playground listening on http://{address}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_valid_source() {
        let response = compile_source(DEFAULT_SOURCE);
        assert!(response.valid, "{:?}", response.diagnostics);
        assert!(response.rust.unwrap().contains("pub struct User"));
    }

    #[test]
    fn returns_shared_diagnostics_for_invalid_source() {
        let response = compile_source("function unsafe(value: any): void {}");
        assert!(!response.valid);
        assert!(
            response
                .diagnostics
                .iter()
                .any(|item| item.code == "SFT001")
        );
    }
}
