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
    Named(String),
    Array(Box<Type>),
    Optional(Box<Type>),
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
    pub params: Vec<Parameter>,
    pub return_type: Option<Type>,
    pub body: String,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Program {
    pub source: String,
    pub structs: Vec<StructDecl>,
    pub enums: Vec<EnumDecl>,
    pub functions: Vec<FunctionDecl>,
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
        structs: Vec::new(),
        enums: Vec::new(),
        functions: Vec::new(),
    };

    let mut cursor = 0;
    while cursor < source.len() {
        cursor = skip_space_and_comments(source, cursor);
        let start = cursor;
        let (keyword, after_keyword) = next_word(source, cursor);
        let after_keyword = if keyword == "export" {
            let position = skip_space_and_comments(source, after_keyword);
            next_word(source, position).1
        } else {
            after_keyword
        };
        let actual_keyword = if keyword == "export" {
            next_word(
                source,
                skip_space_and_comments(source, cursor + keyword.len()),
            )
            .0
        } else {
            keyword
        };

        match actual_keyword {
            "type" => {
                let (declaration, end) = parse_struct(source, start, after_keyword, false)?;
                if let Some(declaration) = declaration {
                    program.structs.push(declaration);
                }
                cursor = end;
            }
            "interface" => {
                let (declaration, end) = parse_struct(source, start, after_keyword, true)?;
                if let Some(declaration) = declaration {
                    program.structs.push(declaration);
                }
                cursor = end;
            }
            "enum" => {
                let (declaration, end) = parse_enum(source, start, after_keyword)?;
                program.enums.push(declaration);
                cursor = end;
            }
            "function" => {
                let (declaration, end) = parse_function(source, start, after_keyword)?;
                program.functions.push(declaration);
                cursor = end;
            }
            _ => cursor = advance_top_level(source, cursor),
        }
    }
    Ok(program)
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
    Ok((
        FunctionDecl {
            name: name.to_owned(),
            params,
            return_type,
            body: source[cursor + 1..body_end].trim().to_owned(),
            span: Span {
                start,
                end: body_end + 1,
            },
        },
        body_end + 1,
    ))
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
        "any" | "unknown" | "null" | "undefined" => Type::Unsupported(input.to_owned()),
        value if value.is_empty() => Type::Unsupported("<missing>".to_owned()),
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
}
