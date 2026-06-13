use rustify_analyzer::{Diagnostic as RustifyDiagnostic, analyze};
use std::collections::HashMap;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

impl Backend {
    async fn validate(&self, uri: Url, text: String) {
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        let diagnostics = match rustify_parser::parse(&text) {
            Ok(program) => analyze(&program)
                .diagnostics
                .iter()
                .map(|diagnostic| to_lsp_diagnostic(&text, diagnostic))
                .collect(),
            Err(error) => vec![Diagnostic {
                range: Range::default(),
                severity: Some(DiagnosticSeverity::ERROR),
                source: Some("rustify".to_owned()),
                message: error.to_string(),
                ..Diagnostic::default()
            }],
        };
        self.client
            .publish_diagnostics(uri, diagnostics, None)
            .await;
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..ServerCapabilities::default()
            },
            server_info: Some(ServerInfo {
                name: "rustify-lsp".to_owned(),
                version: Some(env!("CARGO_PKG_VERSION").to_owned()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Rustify language server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.validate(params.text_document.uri, params.text_document.text)
            .await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.validate(params.text_document.uri, change.text).await;
        }
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&uri) else {
            return Ok(None);
        };
        let word = word_at(text, position);
        let rust_type = match word.as_str() {
            "string" => "String",
            "number" => "f64",
            "boolean" => "bool",
            "console.log" => "println!",
            _ if word.ends_with("[]") => "Vec<T>",
            _ => return Ok(None),
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**Rust target:** `{rust_type}`"),
            }),
            range: None,
        }))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Ok(program) = rustify_parser::parse(text) else {
            return Ok(None);
        };
        let symbols = program
            .structs
            .iter()
            .map(|item| symbol(text, &item.name, item.span, SymbolKind::STRUCT))
            .chain(
                program
                    .enums
                    .iter()
                    .map(|item| symbol(text, &item.name, item.span, SymbolKind::ENUM)),
            )
            .chain(
                program
                    .functions
                    .iter()
                    .map(|item| symbol(text, &item.name, item.span, SymbolKind::FUNCTION)),
            )
            .collect();
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let actions = params
            .context
            .diagnostics
            .into_iter()
            .filter(|diagnostic| {
                diagnostic.code == Some(NumberOrString::String("SFT001".to_owned()))
            })
            .map(|diagnostic| {
                CodeActionOrCommand::CodeAction(CodeAction {
                    title: "Replace `any` with `string`".to_owned(),
                    kind: Some(CodeActionKind::QUICKFIX),
                    diagnostics: Some(vec![diagnostic.clone()]),
                    edit: Some(WorkspaceEdit {
                        changes: Some(HashMap::from([(
                            params.text_document.uri.clone(),
                            vec![TextEdit {
                                range: diagnostic.range,
                                new_text: ": string".to_owned(),
                            }],
                        )])),
                        ..WorkspaceEdit::default()
                    }),
                    ..CodeAction::default()
                })
            })
            .collect();
        Ok(Some(actions))
    }
}

fn to_lsp_diagnostic(source: &str, diagnostic: &RustifyDiagnostic) -> Diagnostic {
    Diagnostic {
        range: Range::new(
            position(source, diagnostic.span.start),
            position(source, diagnostic.span.end),
        ),
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(NumberOrString::String(diagnostic.code.to_owned())),
        source: Some("rustify".to_owned()),
        message: match &diagnostic.hint {
            Some(hint) => format!("{}\nSuggestion: {hint}", diagnostic.message),
            None => diagnostic.message.clone(),
        },
        ..Diagnostic::default()
    }
}

fn position(source: &str, offset: usize) -> Position {
    let prefix = &source[..offset.min(source.len())];
    Position::new(
        prefix.bytes().filter(|byte| *byte == b'\n').count() as u32,
        prefix.rsplit('\n').next().map(str::len).unwrap_or(0) as u32,
    )
}

fn symbol(
    source: &str,
    name: &str,
    span: rustify_parser::Span,
    kind: SymbolKind,
) -> DocumentSymbol {
    #[allow(deprecated)]
    DocumentSymbol {
        name: name.to_owned(),
        detail: Some("Rustify declaration".to_owned()),
        kind,
        tags: None,
        deprecated: None,
        range: Range::new(position(source, span.start), position(source, span.end)),
        selection_range: Range::new(position(source, span.start), position(source, span.end)),
        children: None,
    }
}

fn word_at(source: &str, position: Position) -> String {
    let line = source
        .lines()
        .nth(position.line as usize)
        .unwrap_or_default();
    let column = (position.character as usize).min(line.len());
    let start = line[..column]
        .rfind(|character: char| !(character.is_alphanumeric() || "._[]".contains(character)))
        .map(|value| value + 1)
        .unwrap_or(0);
    let end = line[column..]
        .find(|character: char| !(character.is_alphanumeric() || "._[]".contains(character)))
        .map(|value| column + value)
        .unwrap_or(line.len());
    line[start..end].to_owned()
}

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        documents: RwLock::new(HashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
