use rustify_analyzer::{Diagnostic as RustifyDiagnostic, analyze};
use rustify_parser::{Program, Type};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,
    documents: RwLock<HashMap<Url, String>>,
}

const SEMANTIC_TOKEN_TYPES: [SemanticTokenType; 5] = [
    SemanticTokenType::STRUCT,
    SemanticTokenType::ENUM,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::PROPERTY,
];

impl Backend {
    async fn validate(&self, uri: Url, text: String) {
        self.documents
            .write()
            .await
            .insert(uri.clone(), text.clone());
        let documents = self.documents.read().await.clone();
        let diagnostics = match workspace_program(&uri, &text, &documents) {
            Ok(program) => analyze(&program)
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.span.start < text.len())
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

fn workspace_program(
    uri: &Url,
    text: &str,
    documents: &HashMap<Url, String>,
) -> std::result::Result<Program, rustify_parser::ParseError> {
    let mut program = rustify_parser::parse(text)?;
    let mut visited = HashSet::new();
    if let Ok(path) = uri.to_file_path() {
        visited.insert(path.clone());
        add_imported_declarations(&mut program, &path, documents, &mut visited)?;
    }
    Ok(program)
}

fn add_imported_declarations(
    target: &mut Program,
    importer: &Path,
    documents: &HashMap<Url, String>,
    visited: &mut HashSet<PathBuf>,
) -> std::result::Result<(), rustify_parser::ParseError> {
    let imports = target.imports.clone();
    for import in imports {
        let Some(path) = resolve_lsp_import(importer, &import.source) else {
            continue;
        };
        let canonical = path.canonicalize().unwrap_or(path);
        if !visited.insert(canonical.clone()) {
            continue;
        }
        let source = Url::from_file_path(&canonical)
            .ok()
            .and_then(|uri| documents.get(&uri).cloned())
            .or_else(|| std::fs::read_to_string(&canonical).ok());
        let Some(source) = source else {
            continue;
        };
        let mut imported = rustify_parser::parse(&source)?;
        add_imported_declarations(&mut imported, &canonical, documents, visited)?;
        let offset = target.source.len() + 1;
        shift_program_spans(&mut imported, offset);
        target.source.push('\n');
        target.source.push_str(&imported.source);
        target
            .unsupported_top_level
            .extend(imported.unsupported_top_level);
        target.structs.extend(imported.structs);
        target.enums.extend(imported.enums);
        target.functions.extend(imported.functions);
    }
    Ok(())
}

fn resolve_lsp_import(importer: &Path, specifier: &str) -> Option<PathBuf> {
    if !specifier.starts_with('.') {
        return None;
    }
    let base = importer.parent()?;
    let path = base.join(specifier);
    if path.extension().is_some() {
        return Some(path);
    }
    [path.with_extension("ts"), path.join("index.ts")]
        .into_iter()
        .find(|candidate| candidate.is_file())
}

fn shift_program_spans(program: &mut Program, offset: usize) {
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
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Right(RenameOptions {
                    prepare_provider: Some(true),
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                })),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["rustify.preview".to_owned()],
                    work_done_progress_options: WorkDoneProgressOptions::default(),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensOptions {
                        work_done_progress_options: WorkDoneProgressOptions::default(),
                        legend: SemanticTokensLegend {
                            token_types: SEMANTIC_TOKEN_TYPES.to_vec(),
                            token_modifiers: Vec::new(),
                        },
                        range: None,
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                    }
                    .into(),
                ),
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
        let Some(contents) = hover_contents(text, &word) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: contents,
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

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: semantic_tokens(text),
        })))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let documents = self.documents.read().await;
        let Some(source) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(dynamic_type_quick_fixes(
            &params.text_document.uri,
            source,
            params.context.diagnostics,
        )))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let position = params.text_document_position_params.position;
        let uri = &params.text_document_position_params.text_document.uri;
        let documents = self.documents.read().await;
        let Some(text) = documents.get(uri) else {
            return Ok(None);
        };
        let word = word_at(text, position);
        if let Some(location) = imported_definition(uri, text, &word, &documents) {
            return Ok(Some(GotoDefinitionResponse::Scalar(location)));
        }
        for (uri, source) in documents.iter() {
            if let Some(range) = declaration_range(source, &word) {
                return Ok(Some(GotoDefinitionResponse::Scalar(Location::new(
                    uri.clone(),
                    range,
                ))));
            }
        }
        Ok(None)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let open_documents = self.documents.read().await;
        let documents = workspace_documents(&open_documents);
        let Some(text) = documents.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        let word = word_at(text, params.text_document_position.position);
        let locations = documents
            .iter()
            .flat_map(|(uri, source)| {
                identifier_ranges(source, &word)
                    .into_iter()
                    .map(|range| Location::new(uri.clone(), range))
            })
            .collect();
        Ok(Some(locations))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        if !valid_identifier(&params.new_name) {
            return Err(tower_lsp::jsonrpc::Error::invalid_params(
                "Rustify rename target must be a valid identifier",
            ));
        }
        let open_documents = self.documents.read().await;
        let documents = workspace_documents(&open_documents);
        let Some(text) = documents.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        let word = word_at(text, params.text_document_position.position);
        let changes = documents
            .iter()
            .filter_map(|(uri, source)| {
                let edits: Vec<_> = identifier_ranges(source, &word)
                    .into_iter()
                    .map(|range| TextEdit {
                        range,
                        new_text: params.new_name.clone(),
                    })
                    .collect();
                (!edits.is_empty()).then(|| (uri.clone(), edits))
            })
            .collect();
        Ok(Some(WorkspaceEdit {
            changes: Some(changes),
            ..WorkspaceEdit::default()
        }))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let documents = self.documents.read().await;
        let Some(text) = documents.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Some((word, range)) = word_and_range_at(text, params.position) else {
            return Ok(None);
        };
        if matches!(
            word.as_str(),
            "string" | "number" | "boolean" | "void" | "null" | "undefined"
        ) {
            return Ok(None);
        }
        Ok(Some(PrepareRenameResponse::RangeWithPlaceholder {
            range,
            placeholder: word,
        }))
    }

    async fn symbol(
        &self,
        params: WorkspaceSymbolParams,
    ) -> Result<Option<Vec<SymbolInformation>>> {
        let open_documents = self.documents.read().await;
        let documents = workspace_documents(&open_documents);
        Ok(Some(workspace_symbols(&documents, &params.query)))
    }

    async fn execute_command(
        &self,
        params: ExecuteCommandParams,
    ) -> Result<Option<serde_json::Value>> {
        if params.command != "rustify.preview" {
            return Ok(None);
        }
        let uri = params
            .arguments
            .first()
            .and_then(serde_json::Value::as_str)
            .and_then(|value| Url::parse(value).ok())
            .ok_or_else(|| {
                tower_lsp::jsonrpc::Error::invalid_params(
                    "rustify.preview requires a document URI argument",
                )
            })?;
        let documents = self.documents.read().await;
        let rust = preview_translation(&uri, &documents)
            .map_err(tower_lsp::jsonrpc::Error::invalid_params)?;
        Ok(Some(serde_json::Value::String(rust)))
    }
}

fn preview_translation(
    uri: &Url,
    documents: &HashMap<Url, String>,
) -> std::result::Result<String, String> {
    let text = documents
        .get(uri)
        .ok_or_else(|| "document is not open in Rustify LSP".to_owned())?;
    let program = workspace_program(uri, text, documents).map_err(|error| error.to_string())?;
    let analysis = analyze(&program);
    if !analysis.is_valid() {
        return Err(analysis
            .diagnostics
            .iter()
            .map(|diagnostic| format!("[{}] {}", diagnostic.code, diagnostic.message))
            .collect::<Vec<_>>()
            .join("\n"));
    }
    rustify_codegen_rust::emit(analysis.ir.as_ref().expect("valid analysis has IR"))
        .map_err(|error| error.to_string())
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
    let mut offset = offset.min(source.len());
    while !source.is_char_boundary(offset) {
        offset -= 1;
    }
    let prefix = &source[..offset];
    Position::new(
        prefix.bytes().filter(|byte| *byte == b'\n').count() as u32,
        prefix
            .rsplit('\n')
            .next()
            .map(utf16_len)
            .unwrap_or_default(),
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
    word_and_range_at(source, position)
        .map(|(word, _)| word)
        .unwrap_or_default()
}

fn word_and_range_at(source: &str, position_value: Position) -> Option<(String, Range)> {
    let line = source
        .lines()
        .nth(position_value.line as usize)
        .unwrap_or_default();
    let column = byte_offset_at_utf16_column(line, position_value.character);
    let start = line[..column]
        .rfind(|character: char| !is_identifier_character(character))
        .map(|value| value + 1)
        .unwrap_or(0);
    let end = line[column..]
        .find(|character: char| !is_identifier_character(character))
        .map(|value| column + value)
        .unwrap_or(line.len());
    (start < end).then(|| {
        (
            line[start..end].to_owned(),
            Range::new(
                Position::new(position_value.line, utf16_len(&line[..start])),
                Position::new(position_value.line, utf16_len(&line[..end])),
            ),
        )
    })
}

fn hover_contents(source: &str, word: &str) -> Option<String> {
    let primitive = match word {
        "string" => Some("String"),
        "number" => Some("f64"),
        "boolean" => Some("bool"),
        "void" => Some("()"),
        "console.log" => Some("println!"),
        _ if word.ends_with("[]") => Some("Vec<T>"),
        _ => None,
    };
    if let Some(target) = primitive {
        return Some(format!("**Rust target:** `{target}`"));
    }
    let program = rustify_parser::parse(source).ok()?;
    if let Some(structure) = program.structs.iter().find(|item| item.name == word) {
        let fields = structure
            .fields
            .iter()
            .map(|field| {
                let ty = if field.optional {
                    format!("Option<{}>", rust_type(&field.ty))
                } else {
                    rust_type(&field.ty)
                };
                format!("pub {}: {}", field.name, ty)
            })
            .collect::<Vec<_>>()
            .join(", ");
        return Some(format!("**Rust struct:** `{word} {{ {fields} }}`"));
    }
    if let Some(enumeration) = program.enums.iter().find(|item| item.name == word) {
        return Some(format!(
            "**Rust enum:** `{word} {{ {} }}`",
            enumeration.variants.join(", ")
        ));
    }
    let function = program.functions.iter().find(|item| item.name == word)?;
    let params = function
        .params
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name,
                parameter
                    .ty
                    .as_ref()
                    .map(rust_type)
                    .unwrap_or_else(|| "<missing>".to_owned())
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    let return_type = match function.return_type.as_ref() {
        Some(Type::Promise(inner)) if function.is_async => rust_type(inner),
        Some(ty) => rust_type(ty),
        None => "<missing>".to_owned(),
    };
    Some(format!(
        "**Rust function:** `pub {}fn {}({params}) -> {return_type}`",
        if function.is_async { "async " } else { "" },
        function.name
    ))
}

fn rust_type(ty: &Type) -> String {
    match ty {
        Type::String => "String".to_owned(),
        Type::Number => "f64".to_owned(),
        Type::Boolean => "bool".to_owned(),
        Type::Void => "()".to_owned(),
        Type::JsonValue => "rustify_runtime::JsonValue".to_owned(),
        Type::Named(name) | Type::Unsupported(name) => name.clone(),
        Type::Array(inner) => format!("Vec<{}>", rust_type(inner)),
        Type::Optional(inner) => format!("Option<{}>", rust_type(inner)),
        Type::Result(ok, error) => format!("Result<{}, {}>", rust_type(ok), rust_type(error)),
        Type::Promise(inner) => format!("impl Future<Output = {}>", rust_type(inner)),
    }
}

fn declaration_range(source: &str, name: &str) -> Option<Range> {
    let program = rustify_parser::parse(source).ok()?;
    let span = program
        .structs
        .iter()
        .find(|item| item.name == name)
        .map(|item| item.span)
        .or_else(|| {
            program
                .enums
                .iter()
                .find(|item| item.name == name)
                .map(|item| item.span)
        })
        .or_else(|| {
            program
                .functions
                .iter()
                .find(|item| item.name == name)
                .map(|item| item.span)
        })?;
    let start = source[span.start..span.end].find(name)? + span.start;
    Some(Range::new(
        position(source, start),
        position(source, start + name.len()),
    ))
}

fn imported_definition(
    uri: &Url,
    source: &str,
    name: &str,
    documents: &HashMap<Url, String>,
) -> Option<Location> {
    let importer = uri.to_file_path().ok()?;
    let program = rustify_parser::parse(source).ok()?;
    let import = program
        .imports
        .iter()
        .find(|import| import.names.iter().any(|imported| imported == name))?;
    let path = resolve_lsp_import(&importer, &import.source)?;
    let canonical = path.canonicalize().unwrap_or(path);
    let target_uri = Url::from_file_path(&canonical).ok()?;
    let target_source = documents
        .get(&target_uri)
        .cloned()
        .or_else(|| std::fs::read_to_string(canonical).ok())?;
    declaration_range(&target_source, name).map(|range| Location::new(target_uri, range))
}

fn identifier_ranges(source: &str, name: &str) -> Vec<Range> {
    if name.is_empty() {
        return Vec::new();
    }
    source
        .match_indices(name)
        .filter(|(start, _)| {
            let before = source[..*start].chars().next_back();
            let after = source[*start + name.len()..].chars().next();
            !before.is_some_and(is_identifier_character)
                && !after.is_some_and(is_identifier_character)
        })
        .map(|(start, _)| {
            Range::new(
                position(source, start),
                position(source, start + name.len()),
            )
        })
        .collect()
}

fn is_identifier_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '$')
}

fn valid_identifier(name: &str) -> bool {
    let mut characters = name.chars();
    characters
        .next()
        .is_some_and(|character| character.is_alphabetic() || matches!(character, '_' | '$'))
        && characters.all(is_identifier_character)
}

fn workspace_documents(open_documents: &HashMap<Url, String>) -> HashMap<Url, String> {
    let mut documents = open_documents.clone();
    let mut pending: Vec<_> = documents.keys().cloned().collect();
    let mut visited = HashSet::new();
    while let Some(uri) = pending.pop() {
        if !visited.insert(uri.clone()) {
            continue;
        }
        let Some(source) = documents.get(&uri).cloned() else {
            continue;
        };
        let Ok(path) = uri.to_file_path() else {
            continue;
        };
        let Ok(program) = rustify_parser::parse(&source) else {
            continue;
        };
        for import in program.imports {
            let Some(imported_path) = resolve_lsp_import(&path, &import.source) else {
                continue;
            };
            let canonical = imported_path.canonicalize().unwrap_or(imported_path);
            let Ok(imported_uri) = Url::from_file_path(canonical.clone()) else {
                continue;
            };
            if !documents.contains_key(&imported_uri)
                && let Ok(imported_source) = std::fs::read_to_string(canonical)
            {
                documents.insert(imported_uri.clone(), imported_source);
            }
            pending.push(imported_uri);
        }
    }
    documents
}

fn workspace_symbols(documents: &HashMap<Url, String>, query: &str) -> Vec<SymbolInformation> {
    let query = query.to_ascii_lowercase();
    let mut symbols = Vec::new();
    for (uri, source) in documents {
        let Ok(program) = rustify_parser::parse(source) else {
            continue;
        };
        for (name, span, kind) in program
            .structs
            .into_iter()
            .map(|item| (item.name, item.span, SymbolKind::STRUCT))
            .chain(
                program
                    .enums
                    .into_iter()
                    .map(|item| (item.name, item.span, SymbolKind::ENUM)),
            )
            .chain(
                program
                    .functions
                    .into_iter()
                    .map(|item| (item.name, item.span, SymbolKind::FUNCTION)),
            )
        {
            if !name.to_ascii_lowercase().contains(&query) {
                continue;
            }
            #[allow(deprecated)]
            symbols.push(SymbolInformation {
                name,
                kind,
                tags: None,
                deprecated: None,
                location: Location::new(
                    uri.clone(),
                    Range::new(position(source, span.start), position(source, span.end)),
                ),
                container_name: None,
            });
        }
    }
    symbols
}

fn semantic_tokens(source: &str) -> Vec<SemanticToken> {
    let Ok(program) = rustify_parser::parse(source) else {
        return Vec::new();
    };
    let mut tokens = Vec::new();
    for structure in &program.structs {
        push_semantic_token(source, &structure.name, structure.span, 0, &mut tokens);
        for field in &structure.fields {
            for range in identifier_ranges(
                &source[structure.span.start..structure.span.end],
                &field.name,
            ) {
                let start = offset_at_position(
                    &source[structure.span.start..structure.span.end],
                    range.start,
                ) + structure.span.start;
                tokens.push((position(source, start), utf16_len(&field.name), 4));
            }
        }
    }
    for enumeration in &program.enums {
        push_semantic_token(source, &enumeration.name, enumeration.span, 1, &mut tokens);
        for variant in &enumeration.variants {
            push_semantic_token(source, variant, enumeration.span, 2, &mut tokens);
        }
    }
    for function in &program.functions {
        push_semantic_token(source, &function.name, function.span, 3, &mut tokens);
    }
    tokens.sort_by_key(|(position, _, _)| (position.line, position.character));
    let mut previous_line = 0;
    let mut previous_start = 0;
    tokens
        .into_iter()
        .map(|(position, length, token_type)| {
            let delta_line = position.line - previous_line;
            let delta_start = if delta_line == 0 {
                position.character - previous_start
            } else {
                position.character
            };
            previous_line = position.line;
            previous_start = position.character;
            SemanticToken {
                delta_line,
                delta_start,
                length,
                token_type,
                token_modifiers_bitset: 0,
            }
        })
        .collect()
}

fn push_semantic_token(
    source: &str,
    name: &str,
    span: rustify_parser::Span,
    token_type: u32,
    tokens: &mut Vec<(Position, u32, u32)>,
) {
    if let Some(relative) = source[span.start..span.end].find(name) {
        tokens.push((
            position(source, span.start + relative),
            utf16_len(name),
            token_type,
        ));
    }
}

fn offset_at_position(source: &str, position: Position) -> usize {
    let line_start = source
        .lines()
        .take(position.line as usize)
        .map(|line| line.len() + 1)
        .sum::<usize>();
    let line = source[line_start.min(source.len())..]
        .split('\n')
        .next()
        .unwrap_or_default();
    line_start.min(source.len()) + byte_offset_at_utf16_column(line, position.character)
}

fn utf16_len(value: &str) -> u32 {
    value.encode_utf16().count() as u32
}

fn byte_offset_at_utf16_column(value: &str, column: u32) -> usize {
    let mut units = 0_u32;
    for (offset, character) in value.char_indices() {
        if units >= column {
            return offset;
        }
        let next = units + character.len_utf16() as u32;
        if next > column {
            return offset;
        }
        units = next;
    }
    value.len()
}

fn dynamic_type_quick_fixes(
    uri: &Url,
    source: &str,
    diagnostics: Vec<Diagnostic>,
) -> CodeActionResponse {
    diagnostics
        .into_iter()
        .filter_map(|diagnostic| {
            if !matches!(
                diagnostic.code.as_ref(),
                Some(NumberOrString::String(code)) if matches!(code.as_str(), "SFT001" | "SFT002")
            ) {
                return None;
            }
            let inferred = inferred_const_type_at(source, diagnostic.range)?;
            Some(CodeActionOrCommand::CodeAction(CodeAction {
                title: format!("Replace dynamic type with `{inferred}`"),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: Some(vec![diagnostic.clone()]),
                edit: Some(WorkspaceEdit {
                    changes: Some(HashMap::from([(
                        uri.clone(),
                        vec![TextEdit {
                            range: diagnostic.range,
                            new_text: inferred.to_owned(),
                        }],
                    )])),
                    ..WorkspaceEdit::default()
                }),
                ..CodeAction::default()
            }))
        })
        .collect()
}

fn inferred_const_type_at(source: &str, range: Range) -> Option<&'static str> {
    let start = offset_at_position(source, range.start).min(source.len());
    let end = offset_at_position(source, range.end).min(source.len());
    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let prefix = source[line_start..start].trim_start();
    if !prefix.starts_with("const ") || !prefix.trim_end().ends_with(':') {
        return None;
    }
    let initializer = source[end..]
        .split(['\n', ';'])
        .next()?
        .trim()
        .strip_prefix('=')?
        .trim();
    if (initializer.starts_with('"') && initializer.ends_with('"'))
        || (initializer.starts_with('\'') && initializer.ends_with('\''))
        || (initializer.starts_with('`') && initializer.ends_with('`'))
    {
        Some("string")
    } else if matches!(initializer, "true" | "false") {
        Some("boolean")
    } else if initializer.parse::<f64>().is_ok() {
        Some("number")
    } else {
        None
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_declarations_and_identifier_references() {
        let source = "function greet(name: string): string { return name }\ngreet(\"A\")";
        assert!(declaration_range(source, "greet").is_some());
        assert_eq!(identifier_ranges(source, "greet").len(), 2);
    }

    #[test]
    fn analyzes_relative_imports_as_a_workspace() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = root
            .join("examples/modules/main.ts")
            .canonicalize()
            .unwrap();
        let text = std::fs::read_to_string(&entry).unwrap();
        let uri = Url::from_file_path(entry).unwrap();
        let program = workspace_program(&uri, &text, &HashMap::new()).unwrap();
        let analysis = analyze(&program);
        assert!(analysis.is_valid(), "{:?}", analysis.diagnostics);
    }

    #[test]
    fn previews_generated_rust_from_open_document_content() {
        let uri = Url::parse("file:///tmp/rustify-preview.ts").unwrap();
        let documents = HashMap::from([(
            uri.clone(),
            "function greet(name: string): string { return `Hi ${name}` }".to_owned(),
        )]);
        let rust = preview_translation(&uri, &documents).unwrap();
        assert!(rust.contains("pub fn greet(name: String) -> String"));
        assert!(rust.contains("format!(\"Hi {}\", name)"));
    }

    #[test]
    fn preview_rejects_invalid_open_document_content() {
        let uri = Url::parse("file:///tmp/rustify-invalid-preview.ts").unwrap();
        let documents = HashMap::from([(uri.clone(), "console.log(\"runtime\")".to_owned())]);
        let error = preview_translation(&uri, &documents).unwrap_err();
        assert!(error.contains("SFT046"), "{error}");
    }

    #[test]
    fn resolves_imported_definition_from_disk() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = root
            .join("examples/modules/main.ts")
            .canonicalize()
            .unwrap();
        let text = std::fs::read_to_string(&entry).unwrap();
        let uri = Url::from_file_path(entry).unwrap();
        let location = imported_definition(&uri, &text, "greet", &HashMap::new()).unwrap();
        assert!(location.uri.path().ends_with("/examples/modules/models.ts"));
    }

    #[test]
    fn hovers_structs_and_functions_as_rust() {
        let source = "type User = { name: string; nickname?: string }\n\
                      function greet(user: User): string { return user.name }";
        let structure = hover_contents(source, "User").unwrap();
        let function = hover_contents(source, "greet").unwrap();
        assert!(structure.contains("Option<String>"));
        assert!(function.contains("pub fn greet(user: User) -> String"));
    }

    #[test]
    fn hover_matches_native_async_codegen_signature() {
        let source = "async function load(): Promise<string> { return \"ready\" }";
        let function = hover_contents(source, "load").unwrap();
        assert!(
            function.contains("pub async fn load() -> String"),
            "{function}"
        );
    }

    #[test]
    fn produces_semantic_tokens_for_declarations_and_members() {
        let source =
            "type User = { firstName: string }\nenum Status { Active }\nfunction greet(): void {}";
        let tokens = semantic_tokens(source);
        let token_types: Vec<_> = tokens.iter().map(|token| token.token_type).collect();
        assert!(token_types.contains(&0), "{tokens:?}");
        assert!(token_types.contains(&1), "{tokens:?}");
        assert!(token_types.contains(&2), "{tokens:?}");
        assert!(token_types.contains(&3), "{tokens:?}");
        assert!(token_types.contains(&4), "{tokens:?}");
        assert!(tokens.windows(2).all(|pair| {
            pair[1].delta_line > 0
                || pair[1].delta_start > 0
                || pair[0].token_type != pair[1].token_type
        }));
    }

    #[test]
    fn workspace_symbols_and_documents_include_imported_files() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let entry = root
            .join("examples/modules/main.ts")
            .canonicalize()
            .unwrap();
        let uri = Url::from_file_path(&entry).unwrap();
        let documents = HashMap::from([(uri, std::fs::read_to_string(entry).unwrap())]);
        let workspace = workspace_documents(&documents);
        assert_eq!(workspace.len(), 2);
        let symbols = workspace_symbols(&workspace, "greet");
        assert_eq!(symbols.len(), 1);
        assert!(symbols[0].location.uri.path().ends_with("/models.ts"));
        let references: usize = workspace
            .values()
            .map(|source| identifier_ranges(source, "greet").len())
            .sum();
        assert_eq!(references, 3);
    }

    #[test]
    fn computes_prepare_rename_ranges_and_validates_names() {
        let source = "function greet(): void {}";
        let (word, range) = word_and_range_at(source, Position::new(0, 11)).unwrap();
        assert_eq!(word, "greet");
        assert_eq!(range.start, Position::new(0, 9));
        assert_eq!(range.end, Position::new(0, 14));
        assert!(valid_identifier("newName"));
        assert!(!valid_identifier("1invalid"));
        assert!(!valid_identifier("not valid"));
    }

    #[test]
    fn uses_utf16_positions_for_lsp_ranges() {
        let source = "const message = \"🚀 café\"\nfunction greet(): void {}";
        for offset in source
            .char_indices()
            .map(|(offset, _)| offset)
            .chain(std::iter::once(source.len()))
        {
            assert_eq!(offset_at_position(source, position(source, offset)), offset);
        }

        let start = source.find("greet").unwrap();
        let cursor = position(source, start + 2);
        let (word, range) = word_and_range_at(source, cursor).unwrap();
        assert_eq!(word, "greet");
        assert_eq!(range.start, position(source, start));
        assert_eq!(range.end, position(source, start + "greet".len()));

        let unicode_name = "function café(): void {}";
        let tokens = semantic_tokens(unicode_name);
        assert_eq!(tokens[0].length, 4);
    }

    #[test]
    fn offers_safe_dynamic_type_quick_fixes() {
        let uri = Url::parse("file:///tmp/rustify-fix.ts").unwrap();
        for (source, expected) in [
            ("const label: any = 'ready'", "string"),
            ("const count: any = 1", "number"),
            ("const enabled: unknown = true", "boolean"),
        ] {
            let token = if source.contains("any") {
                "any"
            } else {
                "unknown"
            };
            let start = source.find(token).unwrap();
            let diagnostic = Diagnostic {
                range: Range::new(
                    position(source, start),
                    position(source, start + token.len()),
                ),
                code: Some(NumberOrString::String(
                    if token == "any" { "SFT001" } else { "SFT002" }.to_owned(),
                )),
                ..Diagnostic::default()
            };
            let actions = dynamic_type_quick_fixes(&uri, source, vec![diagnostic]);
            let CodeActionOrCommand::CodeAction(action) = &actions[0] else {
                panic!("expected a code action");
            };
            let edit = &action.edit.as_ref().unwrap().changes.as_ref().unwrap()[&uri][0];
            assert_eq!(edit.new_text, expected);
            assert_eq!(
                edit.range,
                Range::new(
                    position(source, start),
                    position(source, start + token.len())
                )
            );
        }
    }

    #[test]
    fn omits_unsafe_dynamic_type_quick_fixes() {
        let uri = Url::parse("file:///tmp/rustify-no-fix.ts").unwrap();
        for source in [
            "function consume(value: any): void {}",
            "let value: any = 1",
            "const value: any = compute()",
        ] {
            let start = source.find("any").unwrap();
            let diagnostic = Diagnostic {
                range: Range::new(position(source, start), position(source, start + 3)),
                code: Some(NumberOrString::String("SFT001".to_owned())),
                ..Diagnostic::default()
            };
            assert!(dynamic_type_quick_fixes(&uri, source, vec![diagnostic]).is_empty());
        }
    }
}
