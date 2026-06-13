use rustify_ir as ir;
use rustify_parser::{Program, Span, Type};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: &'static str,
    pub severity: Severity,
    pub message: String,
    pub hint: Option<String>,
    pub span: Span,
}

impl Diagnostic {
    fn error(
        code: &'static str,
        message: impl Into<String>,
        hint: impl Into<Option<String>>,
        span: Span,
    ) -> Self {
        Self {
            code,
            severity: Severity::Error,
            message: message.into(),
            hint: hint.into(),
            span,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Analysis {
    pub diagnostics: Vec<Diagnostic>,
    pub ir: Option<ir::Program>,
}

impl Analysis {
    pub fn is_valid(&self) -> bool {
        !self
            .diagnostics
            .iter()
            .any(|item| item.severity == Severity::Error)
    }
}

pub fn analyze(program: &Program) -> Analysis {
    let mut diagnostics = validate_forbidden_syntax(&program.source);
    validate_declarations(program, &mut diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|item| item.severity == Severity::Error)
    {
        None
    } else {
        Some(lower(program))
    };
    Analysis { diagnostics, ir }
}

fn validate_forbidden_syntax(source: &str) -> Vec<Diagnostic> {
    let rules = [
        (
            "SFT001",
            ": any",
            "`any` is not supported by Rustify.",
            "Use a concrete type.",
        ),
        (
            "SFT002",
            ": unknown",
            "`unknown` is not supported by Rustify.",
            "Use a concrete type.",
        ),
        (
            "SFT003",
            "eval(",
            "`eval` cannot be compiled to native Rust.",
            "Replace dynamic evaluation with typed code.",
        ),
        (
            "SFT004",
            "new Function(",
            "The Function constructor is not supported.",
            "Declare a typed function.",
        ),
        (
            "SFT005",
            "delete ",
            "Dynamic property deletion is not supported.",
            "Model optional state with Option<T>.",
        ),
        (
            "SFT006",
            ".prototype",
            "Prototype mutation is not supported.",
            "Use typed functions and structs.",
        ),
        (
            "SFT007",
            "namespace ",
            "TypeScript namespaces are not supported.",
            "Use ES modules.",
        ),
        (
            "SFT008",
            "@",
            "Decorators are not supported.",
            "Remove the decorator.",
        ),
    ];
    let mut diagnostics = Vec::new();
    for (code, needle, message, hint) in rules {
        for (start, _) in source.match_indices(needle) {
            diagnostics.push(Diagnostic::error(
                code,
                message,
                Some(hint.to_owned()),
                Span {
                    start,
                    end: start + needle.len(),
                },
            ));
        }
    }
    diagnostics
}

fn validate_declarations(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let mut symbols = HashSet::new();
    for item in program
        .structs
        .iter()
        .map(|value| (&value.name, value.span))
        .chain(program.enums.iter().map(|value| (&value.name, value.span)))
        .chain(
            program
                .functions
                .iter()
                .map(|value| (&value.name, value.span)),
        )
    {
        if !symbols.insert(item.0.clone()) {
            diagnostics.push(Diagnostic::error(
                "SFT020",
                format!("duplicate symbol `{}`", item.0),
                Some("Rename one of the declarations.".to_owned()),
                item.1,
            ));
        }
    }

    for structure in &program.structs {
        for field in &structure.fields {
            validate_type(&field.ty, structure.span, diagnostics);
        }
    }
    for function in &program.functions {
        for parameter in &function.params {
            match &parameter.ty {
                Some(ty) => validate_type(ty, function.span, diagnostics),
                None => diagnostics.push(Diagnostic::error(
                    "SFT011",
                    format!("parameter `{}` requires an explicit type", parameter.name),
                    Some("Add a concrete TypeScript type annotation.".to_owned()),
                    function.span,
                )),
            }
        }
        match &function.return_type {
            Some(ty) => validate_type(ty, function.span, diagnostics),
            None => diagnostics.push(Diagnostic::error(
                "SFT010",
                format!(
                    "function `{}` requires an explicit return type",
                    function.name
                ),
                Some("Add a return type annotation.".to_owned()),
                function.span,
            )),
        }
    }
    validate_function_calls(program, diagnostics);
}

fn validate_function_calls(program: &Program, diagnostics: &mut Vec<Diagnostic>) {
    let functions: HashMap<_, _> = program
        .functions
        .iter()
        .map(|function| (function.name.as_str(), function.params.len()))
        .collect();
    for function in &program.functions {
        for (name, expected) in &functions {
            let needle = format!("{name}(");
            let mut remainder = function.body.as_str();
            while let Some(start) = remainder.find(&needle) {
                let args_start = start + needle.len();
                if let Some(end) = remainder[args_start..].find(')') {
                    let args = &remainder[args_start..args_start + end];
                    let actual = if args.trim().is_empty() {
                        0
                    } else {
                        args.split(',').count()
                    };
                    if actual != *expected {
                        diagnostics.push(Diagnostic::error(
                            "SFT030",
                            format!("`{name}` expects {expected} argument(s), found {actual}"),
                            Some("Pass the declared number of arguments.".to_owned()),
                            function.span,
                        ));
                    }
                    remainder = &remainder[args_start + end + 1..];
                } else {
                    break;
                }
            }
        }
    }
}

fn validate_type(ty: &Type, span: Span, diagnostics: &mut Vec<Diagnostic>) {
    match ty {
        Type::Unsupported(name) => diagnostics.push(Diagnostic::error(
            if name.contains('|') {
                "SFT012"
            } else {
                "SFT013"
            },
            format!("type `{name}` is not supported by Rustify"),
            Some("Use a concrete type or a nullable T | null union.".to_owned()),
            span,
        )),
        Type::Array(inner) | Type::Optional(inner) => validate_type(inner, span, diagnostics),
        _ => {}
    }
}

fn lower(program: &Program) -> ir::Program {
    ir::Program {
        structs: program
            .structs
            .iter()
            .map(|structure| ir::Struct {
                name: structure.name.clone(),
                fields: structure
                    .fields
                    .iter()
                    .map(|field| ir::Field {
                        name: field.name.clone(),
                        ty: if field.optional {
                            ir::Type::Option(Box::new(lower_type(&field.ty)))
                        } else {
                            lower_type(&field.ty)
                        },
                    })
                    .collect(),
            })
            .collect(),
        enums: program
            .enums
            .iter()
            .map(|enumeration| ir::Enum {
                name: enumeration.name.clone(),
                variants: enumeration.variants.clone(),
            })
            .collect(),
        functions: program
            .functions
            .iter()
            .map(|function| ir::Function {
                name: function.name.clone(),
                params: function
                    .params
                    .iter()
                    .map(|parameter| ir::Parameter {
                        name: parameter.name.clone(),
                        ty: lower_type(parameter.ty.as_ref().expect("validated parameter type")),
                    })
                    .collect(),
                return_type: lower_type(
                    function
                        .return_type
                        .as_ref()
                        .expect("validated return type"),
                ),
                body: function.body.clone(),
            })
            .collect(),
    }
}

fn lower_type(ty: &Type) -> ir::Type {
    match ty {
        Type::String => ir::Type::String,
        Type::Number => ir::Type::F64,
        Type::Boolean => ir::Type::Bool,
        Type::Void => ir::Type::Unit,
        Type::Named(name) => ir::Type::Named(name.clone()),
        Type::Array(inner) => ir::Type::Vec(Box::new(lower_type(inner))),
        Type::Optional(inner) => ir::Type::Option(Box::new(lower_type(inner))),
        Type::Unsupported(name) => panic!("unsupported type `{name}` passed validation"),
    }
}

pub fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let prefix = &source[..offset.min(source.len())];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map(str::len).unwrap_or(0) + 1;
    (line, column)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_any_and_missing_types() {
        let program = rustify_parser::parse("function bad(value: any) { return value }").unwrap();
        let analysis = analyze(&program);
        assert!(!analysis.is_valid());
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == "SFT001")
        );
        assert!(
            analysis
                .diagnostics
                .iter()
                .any(|item| item.code == "SFT010")
        );
    }
}
