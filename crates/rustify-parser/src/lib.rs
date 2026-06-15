use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    String,
    Number,
    Boolean,
    Void,
    JsonValue,
    Named(String),
    Array(Box<Type>),
    Optional(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Promise(Box<Type>),
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StructDecl {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumDecl {
    pub name: String,
    pub variants: Vec<String>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub ty: Option<Type>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunctionDecl {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: String,
    pub span: Span,
    #[serde(default)]
    pub is_hybrid: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportBinding {
    pub imported: String,
    pub local: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportDecl {
    pub bindings: Vec<ImportBinding>,
    pub source: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConstDecl {
    pub name: String,
    pub ty: Option<Type>,
    pub value: ConstValue,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConstValue {
    String(String),
    Number(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub source: String,
    pub unsupported_top_level: Vec<Span>,
    pub imports: Vec<ImportDecl>,
    pub reexports: Vec<ImportDecl>,
    pub exports: Vec<String>,
    pub default_export: Option<String>,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<FunctionDecl>,
    pub consts: Vec<ConstDecl>,
}

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("TypeScript syntax error: {0}")]
    Syntax(String),
    #[error("could not parse Rustify declaration near byte {0}")]
    Declaration(usize),
}

pub fn parse(source: &str) -> Result<Program, ParseError> {
    validate_typescript(source)?;
    let mut program = Program {
        source: source.to_owned(),
        unsupported_top_level: Vec::new(),
        imports: Vec::new(),
        reexports: Vec::new(),
        exports: Vec::new(),
        default_export: None,
        structs: Vec::new(),
        enums: Vec::new(),
        functions: Vec::new(),
        consts: Vec::new(),
    };

    let mut cursor = 0;
    while cursor < source.len() {
        cursor = skip_space_and_comments(source, cursor);
        if cursor >= source.len() {
            break;
        }
        let start = cursor;
        let (keyword, after_keyword) = next_word(source, cursor);
        let exported = keyword == "export";
        let declaration_start = if exported {
            skip_space_and_comments(source, after_keyword)
        } else {
            cursor
        };
        if exported && source.as_bytes().get(declaration_start) == Some(&b'{') {
            let (declaration, end) = parse_import(source, start, declaration_start)?;
            program.exports.extend(
                declaration
                    .bindings
                    .iter()
                    .map(|binding| binding.local.clone()),
            );
            program.reexports.push(declaration);
            cursor = end;
            continue;
        }
        let (mut declaration_keyword, mut after_declaration_keyword) =
            next_word(source, declaration_start);
        let default_exported = exported && declaration_keyword == "default";
        if default_exported {
            let default_declaration_start =
                skip_space_and_comments(source, after_declaration_keyword);
            (declaration_keyword, after_declaration_keyword) =
                next_word(source, default_declaration_start);
        }
        let (actual_keyword, after_keyword, is_async) = if declaration_keyword == "async" {
            let function_start = skip_space_and_comments(source, after_declaration_keyword);
            let (actual_keyword, after_keyword) = next_word(source, function_start);
            (actual_keyword, after_keyword, true)
        } else {
            (declaration_keyword, after_declaration_keyword, false)
        };

        match actual_keyword {
            "import" => {
                let (declaration, end) = parse_import(source, start, after_keyword)?;
                program.imports.push(declaration);
                cursor = end;
            }
            "type" => {
                let (declaration, end) = parse_struct(source, start, after_keyword, false)?;
                if let Some(declaration) = declaration {
                    if exported {
                        if default_exported {
                            program.default_export = Some(declaration.name.clone());
                        } else {
                            program.exports.push(declaration.name.clone());
                        }
                    }
                    program.structs.push(declaration);
                } else {
                    program.unsupported_top_level.push(Span { start, end });
                }
                cursor = end;
            }
            "interface" => {
                let (declaration, end) = parse_struct(source, start, after_keyword, true)?;
                if let Some(declaration) = declaration {
                    if exported {
                        if default_exported {
                            program.default_export = Some(declaration.name.clone());
                        } else {
                            program.exports.push(declaration.name.clone());
                        }
                    }
                    program.structs.push(declaration);
                }
                cursor = end;
            }
            "enum" => {
                let (declaration, end) = parse_enum(source, start, after_keyword)?;
                if exported {
                    if default_exported {
                        program.default_export = Some(declaration.name.clone());
                    } else {
                        program.exports.push(declaration.name.clone());
                    }
                }
                program.enums.push(declaration);
                cursor = end;
            }
            "function" => {
                let (declaration, end) = parse_function(source, start, after_keyword, is_async)?;
                if exported {
                    if default_exported {
                        program.default_export = Some(declaration.name.clone());
                    } else {
                        program.exports.push(declaration.name.clone());
                    }
                }
                program.functions.push(declaration);
                cursor = end;
            }
            "const" => {
                let (declaration, end) = parse_const(source, start, after_keyword)?;

                if exported {
                    if default_exported {
                        program.default_export = Some(declaration.name.clone());
                    } else {
                        program.exports.push(declaration.name.clone());
                    }
                }

                program.consts.push(declaration);
                cursor = end;
            }
            _ => {
                cursor = advance_top_level(source, cursor);
                program
                    .unsupported_top_level
                    .push(Span { start, end: cursor });
            }
        }
    }
    Ok(program)
}

fn parse_import(
    source: &str,
    start: usize,
    mut cursor: usize,
) -> Result<(ImportDecl, usize), ParseError> {
    cursor = skip_space_and_comments(source, cursor);
    let bindings = if source.as_bytes().get(cursor) == Some(&b'{') {
        let names_end = matching_delimiter(source, cursor, b'{', b'}')
            .ok_or(ParseError::Declaration(cursor))?;
        let bindings = split_top_level(&source[cursor + 1..names_end], &[','])
            .into_iter()
            .filter_map(|name| {
                let name = name.trim();
                if name.is_empty() {
                    return None;
                }
                let (imported, local) = name
                    .split_once(" as ")
                    .map(|(imported, local)| (imported.trim(), local.trim()))
                    .unwrap_or((name, name));
                Some(ImportBinding {
                    imported: imported.to_owned(),
                    local: local.to_owned(),
                })
            })
            .collect::<Vec<_>>();
        cursor = skip_space_and_comments(source, names_end + 1);
        bindings
    } else {
        let (local, after_local) = next_word(source, cursor);
        if local.is_empty() {
            return Err(ParseError::Declaration(cursor));
        }
        cursor = skip_space_and_comments(source, after_local);
        vec![ImportBinding {
            imported: "default".to_owned(),
            local: local.to_owned(),
        }]
    };
    if bindings
        .iter()
        .any(|binding| binding.imported.is_empty() || binding.local.is_empty())
    {
        return Err(ParseError::Declaration(cursor));
    }
    let (from, after_from) = next_word(source, cursor);
    if from != "from" {
        return Err(ParseError::Declaration(cursor));
    }
    cursor = skip_space_and_comments(source, after_from);
    let quote = *source
        .as_bytes()
        .get(cursor)
        .ok_or(ParseError::Declaration(cursor))?;
    if !matches!(quote, b'\'' | b'"') {
        return Err(ParseError::Declaration(cursor));
    }
    let source_end = source[cursor + 1..]
        .find(quote as char)
        .map(|offset| cursor + offset + 1)
        .ok_or(ParseError::Declaration(cursor))?;
    let end = source[source_end + 1..]
        .find([';', '\n'])
        .map(|offset| source_end + offset + 2)
        .unwrap_or(source_end + 1);
    Ok((
        ImportDecl {
            bindings,
            source: source[cursor + 1..source_end].to_owned(),
            span: Span { start, end },
        },
        end,
    ))
}

fn validate_typescript(source: &str) -> Result<(), ParseError> {
    let allocator = Allocator::default();
    let result = Parser::new(&allocator, source, SourceType::ts()).parse();
    if result.errors.is_empty() {
        Ok(())
    } else {
        Err(ParseError::Syntax(result.errors[0].to_string()))
    }
}

fn parse_struct(
    source: &str,
    start: usize,
    mut cursor: usize,
    interface: bool,
) -> Result<(Option<StructDecl>, usize), ParseError> {
    cursor = skip_space_and_comments(source, cursor);
    let (name, after_name) = next_word(source, cursor);
    if name.is_empty() {
        return Err(ParseError::Declaration(cursor));
    }
    cursor = skip_space_and_comments(source, after_name);
    if !interface {
        if source.as_bytes().get(cursor) != Some(&b'=') {
            return Ok((None, advance_top_level(source, cursor)));
        }
        cursor = skip_space_and_comments(source, cursor + 1);
    }
    if source.as_bytes().get(cursor) != Some(&b'{') {
        return Ok((None, advance_top_level(source, cursor)));
    }
    let end_brace =
        matching_delimiter(source, cursor, b'{', b'}').ok_or(ParseError::Declaration(cursor))?;
    let fields = parse_fields(&source[cursor + 1..end_brace]);
    Ok((
        Some(StructDecl {
            name: name.to_owned(),
            fields,
            span: Span {
                start,
                end: end_brace + 1,
            },
        }),
        end_brace + 1,
    ))
}

fn parse_fields(body: &str) -> Vec<Field> {
    split_top_level(body, &[',', ';', '\n'])
        .into_iter()
        .filter_map(|part| {
            let (left, right) = part.split_once(':')?;
            let left = left.trim();
            let optional = left.ends_with('?');
            let name = left.trim_end_matches('?').trim();
            if name.is_empty() {
                return None;
            }
            Some(Field {
                name: name.to_owned(),
                ty: parse_type(right.trim()),
                optional,
            })
        })
        .collect()
}

fn parse_enum(
    source: &str,
    start: usize,
    mut cursor: usize,
) -> Result<(EnumDecl, usize), ParseError> {
    cursor = skip_space_and_comments(source, cursor);
    let (name, after_name) = next_word(source, cursor);
    cursor = skip_space_and_comments(source, after_name);
    let end_brace =
        matching_delimiter(source, cursor, b'{', b'}').ok_or(ParseError::Declaration(cursor))?;
    let variants = split_top_level(&source[cursor + 1..end_brace], &[',', '\n'])
        .into_iter()
        .filter_map(|variant| {
            let value = variant.split('=').next()?.trim();
            (!value.is_empty()).then(|| value.to_owned())
        })
        .collect();
    Ok((
        EnumDecl {
            name: name.to_owned(),
            variants,
            span: Span {
                start,
                end: end_brace + 1,
            },
        },
        end_brace + 1,
    ))
}

fn parse_function(
    source: &str,
    start: usize,
    mut cursor: usize,
    is_async: bool,
) -> Result<(FunctionDecl, usize), ParseError> {
    cursor = skip_space_and_comments(source, cursor);
    let (name, after_name) = next_word(source, cursor);
    cursor = skip_space_and_comments(source, after_name);
    let params_end =
        matching_delimiter(source, cursor, b'(', b')').ok_or(ParseError::Declaration(cursor))?;
    let params = split_top_level(&source[cursor + 1..params_end], &[','])
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            let (name, ty) = part.split_once(':').unwrap_or((part, ""));
            Parameter {
                name: name.trim().to_owned(),
                ty: (!ty.trim().is_empty()).then(|| parse_type(ty.trim())),
            }
        })
        .collect();
    cursor = skip_space_and_comments(source, params_end + 1);
    let return_type = if source.as_bytes().get(cursor) == Some(&b':') {
        let type_start = skip_space_and_comments(source, cursor + 1);
        let body_start = source[type_start..]
            .find('{')
            .map(|offset| type_start + offset)
            .ok_or(ParseError::Declaration(type_start))?;
        cursor = body_start;
        Some(parse_type(source[type_start..body_start].trim()))
    } else {
        None
    };
    cursor = skip_space_and_comments(source, cursor);
    let body_end =
        matching_delimiter(source, cursor, b'{', b'}').ok_or(ParseError::Declaration(cursor))?;
    let search_limit = start.saturating_sub(200);
    let is_hybrid = source[search_limit..start].contains("@hybrid");

    Ok((
        FunctionDecl {
            name: name.to_owned(),
            is_async,
            params,
            return_type,
            body: source[cursor + 1..body_end].trim().to_owned(),
            span: Span {
                start,
                end: body_end + 1,
            },
            is_hybrid,
        },
        body_end + 1,
    ))
}

fn parse_const(
    source: &str,
    start: usize,
    mut cursor: usize,
) -> Result<(ConstDecl, usize), ParseError> {
    cursor = skip_space_and_comments(source, cursor);

    let (name, after_name) = next_word(source, cursor);

    if name.is_empty() {
        return Err(ParseError::Declaration(cursor));
    }

    cursor = skip_space_and_comments(source, after_name);

    // Check for type annotation (optional)
    let ty = if source.as_bytes().get(cursor) == Some(&b':') {
        let type_start = skip_space_and_comments(source, cursor + 1);
        let eq_sign = source[type_start..]
            .find('=')
            .map(|offset| type_start + offset)
            .ok_or(ParseError::Declaration(type_start))?;
        cursor = eq_sign;

        Some(parse_type(source[type_start..eq_sign].trim()))
    } else {
        None
    };

    // Check for '='
    if source.as_bytes().get(cursor) != Some(&b'=') {
        return Err(ParseError::Declaration(cursor));
    }

    cursor = skip_space_and_comments(source, cursor + 1);

    // Parse the literal value up to a semicolon or newline
    let stmt_end = source[cursor..]
        .find([';', '\n'])
        .map(|offset| cursor + offset)
        .unwrap_or(source.len());

    let val_str = source[cursor..stmt_end].trim();
    let value = parse_const_value(val_str).ok_or(ParseError::Declaration(cursor))?;

    let end = if source.as_bytes().get(stmt_end) == Some(&b';') {
        stmt_end + 1
    } else {
        stmt_end
    };

    Ok((
        ConstDecl {
            name: name.to_owned(),
            ty,
            value,
            span: Span { start, end },
        },
        end,
    ))
}

fn parse_const_value(val_str: &str) -> Option<ConstValue> {
    if val_str == "true" {
        return Some(ConstValue::Boolean(true));
    }

    if val_str == "false" {
        return Some(ConstValue::Boolean(false));
    }

    // Check for string literals (supporting double quotes, single quotes, backticks)
    if (val_str.starts_with('"') && val_str.ends_with('"'))
        || (val_str.starts_with('\'') && val_str.ends_with('\''))
        || (val_str.starts_with('`') && val_str.ends_with('`'))
    {
        return Some(ConstValue::String(val_str[1..val_str.len() - 1].to_owned()));
    }

    // Check for numbers
    if let Ok(_num) = val_str.parse::<f64>() {
        return Some(ConstValue::Number(val_str.to_owned()));
    }

    None
}

pub fn parse_type(input: &str) -> Type {
    let input = input.trim();
    if let Some(inner) = input.strip_suffix("[]") {
        return Type::Array(Box::new(parse_type(inner)));
    }
    if let Some(inner) = input
        .strip_prefix("Array<")
        .and_then(|s| s.strip_suffix('>'))
    {
        return Type::Array(Box::new(parse_type(inner)));
    }
    if let Some(inner) = input
        .strip_prefix("Result<")
        .and_then(|value| value.strip_suffix('>'))
    {
        let arguments = split_top_level(inner, &[',']);
        if arguments.len() == 2 {
            return Type::Result(
                Box::new(parse_type(arguments[0])),
                Box::new(parse_type(arguments[1])),
            );
        }
        return Type::Unsupported(input.to_owned());
    }
    if let Some(inner) = input
        .strip_prefix("Promise<")
        .and_then(|value| value.strip_suffix('>'))
    {
        return Type::Promise(Box::new(parse_type(inner)));
    }
    let union = split_top_level(input, &['|']);
    if union.len() == 2 {
        let left = union[0].trim();
        let right = union[1].trim();
        if matches!(right, "null" | "undefined") {
            return Type::Optional(Box::new(parse_type(left)));
        }
        if matches!(left, "null" | "undefined") {
            return Type::Optional(Box::new(parse_type(right)));
        }
        return Type::Unsupported(input.to_owned());
    }
    match input {
        "string" => Type::String,
        "number" => Type::Number,
        "boolean" => Type::Boolean,
        "void" => Type::Void,
        "JsonValue" => Type::JsonValue,
        "any" | "unknown" | "null" | "undefined" => Type::Unsupported(input.to_owned()),
        "" => Type::Unsupported("<missing>".to_owned()),
        value => Type::Named(value.to_owned()),
    }
}

fn skip_space_and_comments(source: &str, mut cursor: usize) -> usize {
    loop {
        while source
            .as_bytes()
            .get(cursor)
            .is_some_and(u8::is_ascii_whitespace)
        {
            cursor += 1;
        }
        if source[cursor..].starts_with("//") {
            cursor = source[cursor..]
                .find('\n')
                .map(|offset| cursor + offset + 1)
                .unwrap_or(source.len());
        } else if source[cursor..].starts_with("/*") {
            cursor = source[cursor + 2..]
                .find("*/")
                .map(|offset| cursor + offset + 4)
                .unwrap_or(source.len());
        } else {
            return cursor;
        }
    }
}

fn next_word(source: &str, cursor: usize) -> (&str, usize) {
    let end = source[cursor..]
        .find(|character: char| {
            !(character.is_alphanumeric() || character == '_' || character == '$')
        })
        .map(|offset| cursor + offset)
        .unwrap_or(source.len());
    (&source[cursor..end], end)
}

fn matching_delimiter(source: &str, start: usize, open: u8, close: u8) -> Option<usize> {
    if source.as_bytes().get(start) != Some(&open) {
        return None;
    }
    let mut depth = 0;
    let mut quote = None;
    let mut escaped = false;
    for (offset, byte) in source.as_bytes()[start..].iter().copied().enumerate() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == active_quote {
                quote = None;
            }
            continue;
        }
        if matches!(byte, b'\'' | b'"' | b'`') {
            quote = Some(byte);
        } else if byte == open {
            depth += 1;
        } else if byte == close {
            depth -= 1;
            if depth == 0 {
                return Some(start + offset);
            }
        }
    }
    None
}

fn split_top_level<'a>(input: &'a str, separators: &[char]) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in input.char_indices() {
        if let Some(active_quote) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active_quote {
                quote = None;
            }
            continue;
        }
        match character {
            '\'' | '"' | '`' => quote = Some(character),
            '(' | '[' | '{' | '<' => depth += 1,
            ')' | ']' | '}' | '>' => depth -= 1,
            value if depth == 0 && separators.contains(&value) => {
                parts.push(&input[start..index]);
                start = index + value.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&input[start..]);
    parts
}

fn advance_top_level(source: &str, cursor: usize) -> usize {
    source[cursor..]
        .find([';', '\n'])
        .map(|offset| cursor + offset + 1)
        .unwrap_or(source.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mvp_declarations() {
        let program = parse(
            "type User = { id: number; name?: string }\n\
             enum State { Active, Inactive }\n\
             function greet(user: User): string { return `Hi ${user.name}` }",
        )
        .unwrap();
        assert_eq!(program.structs[0].fields.len(), 2);
        assert_eq!(program.enums[0].variants.len(), 2);
        assert_eq!(program.functions[0].return_type, Some(Type::String));
    }

    #[test]
    fn parses_named_imports() {
        let program = parse("import { User as Person, greet } from \"./user\"\n").unwrap();
        assert_eq!(program.imports[0].bindings[0].imported, "User");
        assert_eq!(program.imports[0].bindings[0].local, "Person");
        assert_eq!(program.imports[0].bindings[1].imported, "greet");
        assert_eq!(program.imports[0].bindings[1].local, "greet");
        assert_eq!(program.imports[0].source, "./user");
    }

    #[test]
    fn tracks_exported_declarations() {
        let program =
            parse("type Private = { value: string }\nexport function public(): void {}\n").unwrap();
        assert_eq!(program.exports, ["public"]);
    }

    #[test]
    fn parses_named_reexports_with_aliases() {
        let program = parse("export { User, greet as welcome } from \"./user\"\n").unwrap();
        assert_eq!(program.exports, ["User", "welcome"]);
        assert_eq!(program.reexports[0].bindings[0].imported, "User");
        assert_eq!(program.reexports[0].bindings[1].imported, "greet");
        assert_eq!(program.reexports[0].bindings[1].local, "welcome");
    }

    #[test]
    fn parses_default_exports_and_imports() {
        let exported =
            parse("export default function greet(): string { return \"hi\" }\n").unwrap();
        assert_eq!(exported.default_export.as_deref(), Some("greet"));
        assert!(exported.exports.is_empty());

        let imported = parse("import welcome from \"./greet\"\n").unwrap();
        assert_eq!(imported.imports[0].bindings[0].imported, "default");
        assert_eq!(imported.imports[0].bindings[0].local, "welcome");
    }

    #[test]
    fn parses_json_and_result_types() {
        assert_eq!(parse_type("JsonValue"), Type::JsonValue);
        assert_eq!(
            parse_type("Result<JsonValue, string>"),
            Type::Result(Box::new(Type::JsonValue), Box::new(Type::String))
        );
    }

    #[test]
    fn parses_async_functions_and_promises() {
        let program =
            parse("export async function load(): Promise<string> { return \"ready\" }").unwrap();
        assert!(program.functions[0].is_async);
        assert_eq!(
            program.functions[0].return_type,
            Some(Type::Promise(Box::new(Type::String)))
        );
    }

    #[test]
    fn tracks_unsupported_top_level_code() {
        let program = parse("let value: string = \"hello\"\nconsole.log(value)\n").unwrap();
        assert_eq!(program.unsupported_top_level.len(), 2);
    }

    #[test]
    fn parses_global_const_declarations() {
        let program = parse(
            "const appName = \"demo-app\"\n\
             export const timeoutMs: number = 2000;\n\
             const isProd = true;",
        )
        .unwrap();
        assert_eq!(program.consts.len(), 3);

        assert_eq!(program.consts[0].name, "appName");
        assert_eq!(program.consts[0].ty, None);
        assert_eq!(
            program.consts[0].value,
            ConstValue::String("demo-app".to_owned())
        );

        assert_eq!(program.consts[1].name, "timeoutMs");
        assert_eq!(program.consts[1].ty, Some(Type::Number));
        assert_eq!(
            program.consts[1].value,
            ConstValue::Number("2000".to_owned())
        );
        assert!(program.exports.contains(&"timeoutMs".to_owned()));

        assert_eq!(program.consts[2].name, "isProd");
        assert_eq!(program.consts[2].ty, None);
        assert_eq!(program.consts[2].value, ConstValue::Boolean(true));
    }
}
