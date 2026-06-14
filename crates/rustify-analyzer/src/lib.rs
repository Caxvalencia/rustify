use rustify_ir as ir;
use rustify_parser::{Program, Span, Type};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    pub structs: HashMap<String, HashMap<String, Type>>,
    pub enums: HashMap<String, HashSet<String>>,
    pub functions: HashMap<String, FunctionSignature>,
}

#[derive(Debug, Clone)]
pub struct FunctionSignature {
    pub params: Vec<Type>,
    pub return_type: Type,
}

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
    let symbols = build_symbol_table(program, &mut diagnostics);
    validate_declarations(program, &symbols, &mut diagnostics);
    validate_function_bodies(program, &symbols, &mut diagnostics);
    let ir = if diagnostics
        .iter()
        .any(|item| item.severity == Severity::Error)
    {
        None
    } else {
        Some(lower(program, &symbols))
    };
    Analysis { diagnostics, ir }
}

fn validate_forbidden_syntax(source: &str) -> Vec<Diagnostic> {
    let searchable = mask_strings_and_comments(source);
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
        (
            "SFT009",
            "Reflect.",
            "Reflect metadata and dynamic reflection are not supported.",
            "Use statically typed fields and functions.",
        ),
        (
            "SFT014",
            "declare ",
            "Ambient declarations are not supported.",
            "Provide a concrete Rustify implementation.",
        ),
        (
            "SFT015",
            "global {",
            "Global augmentation is not supported.",
            "Use explicit module exports.",
        ),
        (
            "SFT016",
            "this.",
            "Dynamic `this` access is not supported.",
            "Pass a typed value explicitly.",
        ),
        (
            "SFT017",
            "]:",
            "Dynamic index signatures are not supported.",
            "Declare concrete object fields.",
        ),
        (
            "SFT019",
            "with (",
            "`with` statements are not supported.",
            "Use explicit typed identifiers.",
        ),
        (
            "SFT024",
            "Object.defineProperty(",
            "Monkey patching is not supported.",
            "Declare the property in a Rustify type.",
        ),
    ];
    let mut diagnostics = Vec::new();
    for (code, needle, message, hint) in rules {
        for (start, _) in searchable.match_indices(needle) {
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
    for (start, end) in dynamic_property_assignments(&searchable) {
        diagnostics.push(Diagnostic::error(
            "SFT018",
            "Dynamic property assignment is not supported.",
            Some("Declare and assign a concrete field.".to_owned()),
            Span { start, end },
        ));
    }
    diagnostics
}

fn dynamic_property_assignments(source: &str) -> Vec<(usize, usize)> {
    let mut assignments = Vec::new();
    for (close, _) in source.match_indices(']') {
        let Some(open) = source[..close].rfind('[') else {
            continue;
        };
        if source[open + 1..close].trim().is_empty() {
            continue;
        }
        let after = source[close + 1..].trim_start();
        if after.starts_with('=') && !after.starts_with("==") {
            assignments.push((open, close + 1));
        }
    }
    assignments
}

fn mask_strings_and_comments(source: &str) -> String {
    #[derive(Clone, Copy)]
    enum State {
        Code,
        Quote(u8),
        LineComment,
        BlockComment,
    }

    let bytes = source.as_bytes();
    let mut masked = bytes.to_vec();
    let mut state = State::Code;
    let mut index = 0;
    let mut escaped = false;
    while index < bytes.len() {
        state = match state {
            State::Code if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                index += 2;
                State::LineComment
            }
            State::Code if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'*') => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                index += 2;
                State::BlockComment
            }
            State::Code if matches!(bytes[index], b'\'' | b'"' | b'`') => {
                let quote = bytes[index];
                masked[index] = b' ';
                index += 1;
                State::Quote(quote)
            }
            State::Quote(quote) => {
                if bytes[index] == b'\n' {
                    escaped = false;
                } else {
                    masked[index] = b' ';
                }
                if escaped {
                    escaped = false;
                    index += 1;
                    State::Quote(quote)
                } else if bytes[index] == b'\\' {
                    escaped = true;
                    index += 1;
                    State::Quote(quote)
                } else if bytes[index] == quote {
                    index += 1;
                    State::Code
                } else {
                    index += 1;
                    State::Quote(quote)
                }
            }
            State::LineComment if bytes[index] == b'\n' => {
                index += 1;
                State::Code
            }
            State::LineComment => {
                masked[index] = b' ';
                index += 1;
                State::LineComment
            }
            State::BlockComment if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') => {
                masked[index] = b' ';
                masked[index + 1] = b' ';
                index += 2;
                State::Code
            }
            State::BlockComment => {
                if bytes[index] != b'\n' {
                    masked[index] = b' ';
                }
                index += 1;
                State::BlockComment
            }
            State::Code => {
                index += 1;
                State::Code
            }
        };
    }
    String::from_utf8(masked).expect("mask preserves UTF-8")
}

fn build_symbol_table(program: &Program, diagnostics: &mut Vec<Diagnostic>) -> SymbolTable {
    let mut table = SymbolTable::default();
    table.functions.insert(
        "JSON.parse".to_owned(),
        FunctionSignature {
            params: vec![Type::String],
            return_type: Type::Result(Box::new(Type::JsonValue), Box::new(Type::String)),
        },
    );
    table.functions.insert(
        "JSON.stringify".to_owned(),
        FunctionSignature {
            params: vec![Type::JsonValue],
            return_type: Type::Result(Box::new(Type::String), Box::new(Type::String)),
        },
    );
    table.functions.insert(
        "JSON.stringifyPretty".to_owned(),
        FunctionSignature {
            params: vec![Type::JsonValue],
            return_type: Type::Result(Box::new(Type::String), Box::new(Type::String)),
        },
    );
    table.functions.insert(
        "Rustify.sleep".to_owned(),
        FunctionSignature {
            params: vec![Type::Number],
            return_type: Type::Promise(Box::new(Type::Void)),
        },
    );
    for name in ["Math.abs", "Math.floor", "Math.ceil", "Math.round"] {
        table.functions.insert(
            name.to_owned(),
            FunctionSignature {
                params: vec![Type::Number],
                return_type: Type::Number,
            },
        );
    }
    for name in ["Math.min", "Math.max", "Math.pow"] {
        table.functions.insert(
            name.to_owned(),
            FunctionSignature {
                params: vec![Type::Number, Type::Number],
                return_type: Type::Number,
            },
        );
    }
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
    let mut rust_types = HashMap::new();
    for item in program
        .structs
        .iter()
        .map(|value| (&value.name, value.span))
        .chain(program.enums.iter().map(|value| (&value.name, value.span)))
    {
        let rust_name = rust_type_identifier_key(item.0);
        if let Some(previous) = rust_types.insert(rust_name.clone(), item.0.clone())
            && previous != *item.0
        {
            diagnostics.push(rust_name_collision(&previous, item.0, &rust_name, item.1));
        }
    }
    let mut rust_functions = HashMap::new();
    for function in &program.functions {
        let rust_name = rust_identifier_key(&function.name);
        if let Some(previous) = rust_functions.insert(rust_name.clone(), function.name.clone())
            && previous != function.name
        {
            diagnostics.push(rust_name_collision(
                &previous,
                &function.name,
                &rust_name,
                function.span,
            ));
        }
    }

    for structure in &program.structs {
        let mut fields = HashMap::new();
        let mut rust_fields = HashMap::new();
        for field in &structure.fields {
            let field_type = if field.optional {
                Type::Optional(Box::new(field.ty.clone()))
            } else {
                field.ty.clone()
            };
            if fields.insert(field.name.clone(), field_type).is_some() {
                diagnostics.push(Diagnostic::error(
                    "SFT022",
                    format!("duplicate field `{}` in `{}`", field.name, structure.name),
                    Some("Remove or rename the duplicate field.".to_owned()),
                    structure.span,
                ));
            }
            let rust_name = rust_identifier_key(&field.name);
            if let Some(previous) = rust_fields.insert(rust_name.clone(), field.name.clone())
                && previous != field.name
            {
                diagnostics.push(rust_name_collision(
                    &previous,
                    &field.name,
                    &rust_name,
                    structure.span,
                ));
            }
        }
        table.structs.insert(structure.name.clone(), fields);
    }
    for enumeration in &program.enums {
        let mut rust_variants = HashMap::new();
        for variant in &enumeration.variants {
            let rust_name = rust_type_identifier_key(variant);
            if let Some(previous) = rust_variants.insert(rust_name.clone(), variant.clone())
                && previous != *variant
            {
                diagnostics.push(rust_name_collision(
                    &previous,
                    variant,
                    &rust_name,
                    enumeration.span,
                ));
            }
        }
        table.enums.insert(
            enumeration.name.clone(),
            enumeration.variants.iter().cloned().collect(),
        );
    }
    for function in &program.functions {
        if let (Some(return_type), true) = (
            function.return_type.clone(),
            function
                .params
                .iter()
                .all(|parameter| parameter.ty.is_some()),
        ) {
            table.functions.insert(
                function.name.clone(),
                FunctionSignature {
                    params: function
                        .params
                        .iter()
                        .filter_map(|parameter| parameter.ty.clone())
                        .collect(),
                    return_type,
                },
            );
        }
    }
    table
}

fn validate_declarations(
    program: &Program,
    symbols: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for span in &program.unsupported_top_level {
        diagnostics.push(Diagnostic::error(
            "SFT046",
            "top-level executable code or declaration is not supported in native mode",
            Some("Move executable code into a typed function or enable hybrid mode.".to_owned()),
            *span,
        ));
    }
    for import in &program.imports {
        if !import.source.starts_with('.') {
            diagnostics.push(Diagnostic::error(
                "SFT025",
                format!("non-relative import `{}` is not supported", import.source),
                Some("Import a relative Rustify module.".to_owned()),
                import.span,
            ));
        }
    }
    for structure in &program.structs {
        for field in &structure.fields {
            validate_type(&field.ty, structure.span, symbols, diagnostics);
            if type_contains_promise(&field.ty) {
                diagnostics.push(Diagnostic::error(
                    "SFT060",
                    format!(
                        "field `{}` cannot contain Promise<T> in native mode",
                        field.name
                    ),
                    Some("Keep promises as direct async function values.".to_owned()),
                    structure.span,
                ));
            }
        }
    }
    for function in &program.functions {
        let mut parameters = HashSet::new();
        let mut rust_parameters = HashMap::new();
        for parameter in &function.params {
            if !parameters.insert(parameter.name.as_str()) {
                diagnostics.push(Diagnostic::error(
                    "SFT023",
                    format!("duplicate parameter `{}`", parameter.name),
                    Some("Rename one of the parameters.".to_owned()),
                    function.span,
                ));
            }
            let rust_name = rust_identifier_key(&parameter.name);
            if let Some(previous) =
                rust_parameters.insert(rust_name.clone(), parameter.name.clone())
                && previous != parameter.name
            {
                diagnostics.push(rust_name_collision(
                    &previous,
                    &parameter.name,
                    &rust_name,
                    function.span,
                ));
            }
            match &parameter.ty {
                Some(ty) => {
                    validate_type(ty, function.span, symbols, diagnostics);
                    if type_contains_promise(ty) && !is_direct_promise(ty) {
                        diagnostics.push(Diagnostic::error(
                            "SFT060",
                            format!(
                                "parameter `{}` cannot contain nested Promise<T> values",
                                parameter.name
                            ),
                            Some("Use Promise<T> only as a direct parameter type.".to_owned()),
                            function.span,
                        ));
                    }
                }
                None => diagnostics.push(Diagnostic::error(
                    "SFT011",
                    format!("parameter `{}` requires an explicit type", parameter.name),
                    Some("Add a concrete TypeScript type annotation.".to_owned()),
                    function.span,
                )),
            }
        }
        match &function.return_type {
            Some(ty) => {
                validate_type(ty, function.span, symbols, diagnostics);
                if type_contains_promise(ty) && !is_direct_promise(ty) {
                    diagnostics.push(Diagnostic::error(
                        "SFT060",
                        format!(
                            "function `{}` cannot return nested Promise<T> values",
                            function.name
                        ),
                        Some("Return one direct Promise<T> from an async function.".to_owned()),
                        function.span,
                    ));
                }
                if function.is_async && !matches!(ty, Type::Promise(_)) {
                    diagnostics.push(Diagnostic::error(
                        "SFT042",
                        format!(
                            "async function `{}` must declare a `Promise<T>` return type",
                            function.name
                        ),
                        Some("Wrap the declared return type in Promise<T>.".to_owned()),
                        function.span,
                    ));
                } else if !function.is_async && matches!(ty, Type::Promise(_)) {
                    diagnostics.push(Diagnostic::error(
                        "SFT043",
                        format!(
                            "function `{}` returns `Promise<T>` but is not async",
                            function.name
                        ),
                        Some(
                            "Add the `async` keyword or use a synchronous return type.".to_owned(),
                        ),
                        function.span,
                    ));
                }
            }
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
}

fn validate_function_bodies(
    program: &Program,
    symbols: &SymbolTable,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for function in &program.functions {
        if !function.is_async && contains_await(&function.body) {
            diagnostics.push(Diagnostic::error(
                "SFT044",
                format!(
                    "`await` can only be used inside async function `{}`",
                    function.name
                ),
                Some("Add `async` and return Promise<T>.".to_owned()),
                function.span,
            ));
        }
        if function_return_value_type(function).is_some_and(|ty| ty != &Type::Void)
            && !body_definitely_returns(&function.body)
        {
            diagnostics.push(Diagnostic::error(
                "SFT063",
                format!(
                    "function `{}` does not return a value on every path",
                    function.name
                ),
                Some("Add a return value after conditional paths or in every branch.".to_owned()),
                function.span,
            ));
        }
        for parameter in &function.params {
            if matches!(parameter.ty, Some(Type::Promise(_))) {
                let uses = identifier_reference_count(&function.body, &parameter.name);
                if uses != 1 {
                    diagnostics.push(Diagnostic::error(
                        "SFT062",
                        format!(
                            "Promise parameter `{}` must be consumed exactly once, found {uses} uses",
                            parameter.name
                        ),
                        Some(
                            "Await or pass the promise exactly once because Rust futures are consumed."
                                .to_owned(),
                        ),
                        function.span,
                    ));
                }
            }
        }
        let mut locals: HashMap<String, Type> = function
            .params
            .iter()
            .filter_map(|parameter| parameter.ty.clone().map(|ty| (parameter.name.clone(), ty)))
            .collect();
        let mut mutable_locals = HashSet::new();
        validate_body(
            &function.body,
            function,
            symbols,
            &mut locals,
            &mut mutable_locals,
            diagnostics,
            0,
        );
    }
}

fn validate_body(
    body: &str,
    function: &rustify_parser::FunctionDecl,
    symbols: &SymbolTable,
    locals: &mut HashMap<String, Type>,
    mutable_locals: &mut HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
    loop_depth: usize,
) {
    for statement in split_statements(body) {
        let statement = statement.trim();
        if let Some(rest) = statement.strip_prefix("const ") {
            validate_variable(
                rest,
                function.span,
                symbols,
                locals,
                mutable_locals,
                diagnostics,
            );
        } else if let Some(rest) = statement.strip_prefix("let ") {
            if let Some(name) = validate_variable(
                rest,
                function.span,
                symbols,
                locals,
                mutable_locals,
                diagnostics,
            ) {
                mutable_locals.insert(name);
            }
        } else if statement == "return" {
            if function_return_value_type(function) != Some(&Type::Void) {
                diagnostics.push(Diagnostic::error(
                    "SFT033",
                    format!("function `{}` must return a value", function.name),
                    Some("Return a value matching the declared return type.".to_owned()),
                    function.span,
                ));
            }
        } else if let Some(expression) = statement.strip_prefix("return ") {
            validate_array_mutation(
                expression,
                locals,
                mutable_locals,
                function.span,
                diagnostics,
            );
            reject_push_value(expression, function.span, diagnostics);
            if let (Some(expected), Some(actual)) = (
                function_return_value_type(function),
                infer_expression(expression, symbols, locals, function.span, diagnostics),
            ) && !types_compatible(expected, &actual)
            {
                diagnostics.push(Diagnostic::error(
                    "SFT033",
                    format!(
                        "function `{}` returns `{}`, expected `{}`",
                        function.name,
                        type_name(&actual),
                        type_name(expected)
                    ),
                    Some("Return a value matching the declared return type.".to_owned()),
                    function.span,
                ));
            }
        } else if matches!(statement, "break" | "continue") {
            if loop_depth == 0 {
                diagnostics.push(Diagnostic::error(
                    "SFT051",
                    format!("`{statement}` can only be used inside a loop"),
                    Some(format!(
                        "Move `{statement}` inside a `while` or `for...of` loop."
                    )),
                    function.span,
                ));
            }
        } else if statement.starts_with("if ")
            || statement.starts_with("while ")
            || statement.starts_with("for ")
        {
            validate_control_flow(
                statement,
                function,
                symbols,
                locals,
                mutable_locals,
                diagnostics,
                loop_depth,
            );
        } else if let Some(argument) = statement
            .strip_prefix("console.log(")
            .and_then(|value| value.strip_suffix(')'))
        {
            for argument in split_arguments(argument) {
                validate_array_mutation(
                    argument,
                    locals,
                    mutable_locals,
                    function.span,
                    diagnostics,
                );
                reject_push_value(argument, function.span, diagnostics);
                if let Some(ty) =
                    infer_expression(argument, symbols, locals, function.span, diagnostics)
                    && !type_is_debuggable(&ty)
                {
                    diagnostics.push(Diagnostic::error(
                        "SFT059",
                        format!(
                            "console.log cannot print `{}` in native mode",
                            type_name(&ty)
                        ),
                        Some("Await promises before logging their resolved value.".to_owned()),
                        function.span,
                    ));
                }
            }
        } else if let Some((name, expression)) = split_assignment(statement) {
            validate_array_mutation(
                expression,
                locals,
                mutable_locals,
                function.span,
                diagnostics,
            );
            reject_push_value(expression, function.span, diagnostics);
            let actual = infer_expression(expression, symbols, locals, function.span, diagnostics);
            match (locals.get(name), actual) {
                (Some(_), _) if !mutable_locals.contains(name) => {
                    diagnostics.push(Diagnostic::error(
                        "SFT055",
                        format!("cannot assign to immutable binding `{name}`"),
                        Some("Declare the binding with `let` before reassigning it.".to_owned()),
                        function.span,
                    ));
                }
                (Some(expected), Some(actual)) if !types_compatible(expected, &actual) => {
                    diagnostics.push(Diagnostic::error(
                        "SFT032",
                        format!(
                            "cannot assign `{}` to `{name}: {}`",
                            type_name(&actual),
                            type_name(expected)
                        ),
                        Some("Assign a value with the declared variable type.".to_owned()),
                        function.span,
                    ));
                }
                (None, _) => diagnostics.push(Diagnostic::error(
                    "SFT031",
                    format!("unknown identifier `{name}`"),
                    Some("Declare the variable before assigning to it.".to_owned()),
                    function.span,
                )),
                _ => {}
            }
        } else if array_mutation_receiver(statement).is_some() {
            validate_array_mutation(
                statement,
                locals,
                mutable_locals,
                function.span,
                diagnostics,
            );
            infer_expression(statement, symbols, locals, function.span, diagnostics);
        } else {
            validate_array_mutation(
                statement,
                locals,
                mutable_locals,
                function.span,
                diagnostics,
            );
            reject_push_value(statement, function.span, diagnostics);
            if infer_expression(statement, symbols, locals, function.span, diagnostics)
                .is_some_and(|ty| matches!(ty, Type::Promise(_)))
            {
                diagnostics.push(Diagnostic::error(
                    "SFT061",
                    "Promise<T> cannot be ignored in native mode",
                    Some("Await the promise so the generated Rust future is executed.".to_owned()),
                    function.span,
                ));
            }
        }
    }
}

fn validate_control_flow(
    statement: &str,
    function: &rustify_parser::FunctionDecl,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
    mutable_locals: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
    loop_depth: usize,
) {
    let Some(block_start) = statement.find('{') else {
        return;
    };
    let Some(block_end) = matching_brace(statement, block_start) else {
        return;
    };
    let header = statement[..block_start].trim();
    let mut nested_locals = locals.clone();
    let mut nested_mutable_locals = mutable_locals.clone();

    let nested_loop_depth = if header.starts_with("while ") || header.starts_with("for ") {
        loop_depth + 1
    } else {
        loop_depth
    };

    if let Some(condition) = header
        .strip_prefix("if ")
        .or_else(|| header.strip_prefix("while "))
    {
        if let Some(actual) = infer_expression(
            strip_parentheses(condition),
            symbols,
            locals,
            function.span,
            diagnostics,
        ) && actual != Type::Boolean
        {
            diagnostics.push(Diagnostic::error(
                "SFT038",
                format!(
                    "control-flow condition must be `boolean`, found `{}`",
                    type_name(&actual)
                ),
                Some("Use an expression that resolves to boolean.".to_owned()),
                function.span,
            ));
        }
    } else if let Some(iteration) = header.strip_prefix("for ") {
        let iteration = strip_parentheses(iteration)
            .trim_start_matches("const ")
            .trim_start_matches("let ");
        if let Some((binding, iterable)) = iteration.split_once(" of ") {
            let binding = binding.trim();
            if let Some(existing) = locals.keys().find(|existing| {
                existing.as_str() != binding
                    && rust_identifier_key(existing) == rust_identifier_key(binding)
            }) {
                diagnostics.push(rust_name_collision(
                    existing,
                    binding,
                    &rust_identifier_key(binding),
                    function.span,
                ));
            }
            match infer_expression(iterable, symbols, locals, function.span, diagnostics) {
                Some(Type::Array(inner)) => {
                    nested_locals.insert(binding.to_owned(), *inner);
                }
                Some(actual) => diagnostics.push(Diagnostic::error(
                    "SFT039",
                    format!(
                        "`for...of` requires an array, found `{}`",
                        type_name(&actual)
                    ),
                    Some("Iterate over a value with type T[].".to_owned()),
                    function.span,
                )),
                None => {}
            }
        }
    }
    validate_body(
        &statement[block_start + 1..block_end],
        function,
        symbols,
        &mut nested_locals,
        &mut nested_mutable_locals,
        diagnostics,
        nested_loop_depth,
    );

    let remainder = statement[block_end + 1..].trim();
    if let Some(else_body) = remainder.strip_prefix("else") {
        let else_body = else_body.trim();
        if else_body.starts_with("if ") {
            validate_control_flow(
                else_body,
                function,
                symbols,
                locals,
                mutable_locals,
                diagnostics,
                loop_depth,
            );
        } else if else_body.starts_with('{')
            && let Some(end) = matching_brace(else_body, 0)
        {
            validate_body(
                &else_body[1..end],
                function,
                symbols,
                &mut locals.clone(),
                &mut mutable_locals.clone(),
                diagnostics,
                loop_depth,
            );
        }
    }
}

fn validate_variable(
    declaration: &str,
    span: Span,
    symbols: &SymbolTable,
    locals: &mut HashMap<String, Type>,
    mutable_locals: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<String> {
    let (left, expression) = declaration.split_once('=')?;
    let (name, annotation) = left
        .split_once(':')
        .map(|(name, ty)| (name.trim(), Some(rustify_parser::parse_type(ty.trim()))))
        .unwrap_or((left.trim(), None));
    if let Some(existing) = locals.keys().find(|existing| {
        existing.as_str() != name && rust_identifier_key(existing) == rust_identifier_key(name)
    }) {
        diagnostics.push(rust_name_collision(
            existing,
            name,
            &rust_identifier_key(name),
            span,
        ));
    }
    validate_array_mutation(expression, locals, mutable_locals, span, diagnostics);
    reject_push_value(expression, span, diagnostics);
    let inferred = infer_expression(expression, symbols, locals, span, diagnostics);
    if inferred.is_none() {
        diagnostics.push(Diagnostic::error(
            "SFT040",
            format!("cannot infer initializer type for `{name}`"),
            Some("Use a supported expression with a concrete type.".to_owned()),
            span,
        ));
    }
    if let (Some(expected), Some(actual)) = (&annotation, &inferred)
        && !types_compatible(expected, actual)
    {
        diagnostics.push(Diagnostic::error(
            "SFT032",
            format!(
                "cannot initialize `{name}: {}` with `{}`",
                type_name(expected),
                type_name(actual)
            ),
            Some("Use an initializer matching the declared variable type.".to_owned()),
            span,
        ));
    }
    if annotation
        .as_ref()
        .or(inferred.as_ref())
        .is_some_and(type_contains_promise)
    {
        diagnostics.push(Diagnostic::error(
            "SFT060",
            format!("binding `{name}` cannot store Promise<T> in native mode"),
            Some("Await the promise directly instead of storing it.".to_owned()),
            span,
        ));
    }
    if let Some(ty) = annotation.or(inferred) {
        locals.insert(name.to_owned(), ty);
    }
    Some(name.to_owned())
}

fn infer_expression(
    expression: &str,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    let expression = expression.trim().trim_end_matches(';').trim();
    let expression = strip_expression_parentheses(expression);
    if let Some(inner) = expression.strip_prefix("await ") {
        return match infer_expression(inner, symbols, locals, span, diagnostics)? {
            Type::Promise(output) => Some(*output),
            actual => {
                diagnostics.push(Diagnostic::error(
                    "SFT045",
                    format!(
                        "`await` requires a Promise<T>, found `{}`",
                        type_name(&actual)
                    ),
                    Some("Await a call to an async Rustify function.".to_owned()),
                    span,
                ));
                None
            }
        };
    }
    if let Some((condition, then_value, else_value)) = split_conditional_expression(expression) {
        let condition_type = infer_expression(condition, symbols, locals, span, diagnostics)?;
        if condition_type != Type::Boolean {
            diagnostics.push(Diagnostic::error(
                "SFT052",
                format!(
                    "conditional expression requires a `boolean` condition, found `{}`",
                    type_name(&condition_type)
                ),
                Some("Use a condition that resolves to boolean.".to_owned()),
                span,
            ));
            return None;
        }
        let then_type = infer_expression(then_value, symbols, locals, span, diagnostics)?;
        let else_type = infer_expression(else_value, symbols, locals, span, diagnostics)?;
        return compatible_conditional_type(&then_type, &else_type).or_else(|| {
            diagnostics.push(Diagnostic::error(
                "SFT053",
                format!(
                    "conditional branches have incompatible types `{}` and `{}`",
                    type_name(&then_type),
                    type_name(&else_type)
                ),
                Some("Return compatible values from both conditional branches.".to_owned()),
                span,
            ));
            None
        });
    }
    if split_binary_parts(expression).is_some() {
        return infer_binary_expression(expression, symbols, locals, span, diagnostics);
    }
    if let Some(inner) = expression.strip_prefix('!') {
        let actual = infer_expression(inner, symbols, locals, span, diagnostics)?;
        if actual == Type::Boolean {
            return Some(Type::Boolean);
        }
        diagnostics.push(Diagnostic::error(
            "SFT050",
            format!(
                "operator `!` requires `boolean`, found `{}`",
                type_name(&actual)
            ),
            Some("Negate a boolean expression.".to_owned()),
            span,
        ));
        return None;
    }
    if let Some(inner) = expression.strip_prefix('-')
        && expression.parse::<f64>().is_err()
    {
        let actual = infer_expression(inner, symbols, locals, span, diagnostics)?;
        if actual == Type::Number {
            return Some(Type::Number);
        }
        diagnostics.push(Diagnostic::error(
            "SFT050",
            format!(
                "unary `-` requires `number`, found `{}`",
                type_name(&actual)
            ),
            Some("Negate a numeric expression.".to_owned()),
            span,
        ));
        return None;
    }
    if let Some((array, index)) = split_index_access(expression) {
        let array_type = infer_expression(array, symbols, locals, span, diagnostics)?;
        let Type::Array(element_type) = array_type else {
            diagnostics.push(Diagnostic::error(
                "SFT058",
                format!(
                    "indexed access requires an array, found `{}`",
                    type_name(&array_type)
                ),
                Some("Use bracket access on an array value.".to_owned()),
                span,
            ));
            return None;
        };
        let index_type = infer_expression(index, symbols, locals, span, diagnostics)?;
        if index_type != Type::Number {
            diagnostics.push(Diagnostic::error(
                "SFT058",
                format!(
                    "array index requires `number`, found `{}`",
                    type_name(&index_type)
                ),
                Some("Use a numeric array index.".to_owned()),
                span,
            ));
            return None;
        }
        return Some(Type::Optional(element_type));
    }
    if expression.starts_with('`') && expression.ends_with('`') {
        let mut remainder = &expression[1..expression.len() - 1];
        while let Some(start) = remainder.find("${") {
            let after_start = &remainder[start + 2..];
            let end = after_start.find('}')?;
            let interpolated =
                infer_expression(&after_start[..end], symbols, locals, span, diagnostics)?;
            if !matches!(
                interpolated,
                Type::String | Type::Number | Type::Boolean | Type::JsonValue
            ) {
                diagnostics.push(Diagnostic::error(
                    "SFT059",
                    format!(
                        "template interpolation cannot display `{}` in native mode",
                        type_name(&interpolated)
                    ),
                    Some("Interpolate a string, number, boolean, or JsonValue.".to_owned()),
                    span,
                ));
                return None;
            }
            remainder = &after_start[end + 1..];
        }
        return Some(Type::String);
    }
    if expression.starts_with(['"', '\'']) {
        return Some(Type::String);
    }
    if matches!(expression, "true" | "false") {
        return Some(Type::Boolean);
    }
    if matches!(expression, "null" | "undefined") {
        return Some(Type::Optional(Box::new(Type::Void)));
    }
    if expression.starts_with('[') && expression.ends_with(']') {
        let elements = split_arguments(&expression[1..expression.len() - 1]);
        if elements.is_empty() {
            return Some(Type::Array(Box::new(Type::Void)));
        }
        let mut element_type = None;
        for element in elements {
            let actual = infer_expression(element, symbols, locals, span, diagnostics)?;
            if let Some(expected) = &element_type {
                if !types_compatible(expected, &actual) {
                    diagnostics.push(Diagnostic::error(
                        "SFT037",
                        "array elements must have one compatible type",
                        Some("Use values with the same type in the array.".to_owned()),
                        span,
                    ));
                    return None;
                }
            } else {
                element_type = Some(actual);
            }
        }
        return element_type.map(|ty| Type::Array(Box::new(ty)));
    }
    if let Some(fields) = parse_object_literal(expression) {
        let mut provided = HashSet::new();
        if let Some((duplicate, _)) = fields.iter().find(|(field, _)| !provided.insert(*field)) {
            diagnostics.push(Diagnostic::error(
                "SFT049",
                format!("duplicate object literal field `{duplicate}`"),
                Some("Remove or rename the duplicate field.".to_owned()),
                span,
            ));
            return None;
        }
        let mut matches = Vec::new();
        for (name, declared_fields) in &symbols.structs {
            let required_present = declared_fields.iter().all(|(field, ty)| {
                matches!(ty, Type::Optional(_))
                    || fields.iter().any(|(provided, _)| provided == field)
            });
            let no_unknown_fields = fields
                .iter()
                .all(|(field, _)| declared_fields.contains_key(*field));
            if !required_present || !no_unknown_fields {
                continue;
            }
            let mut compatible = true;
            for (field, value) in &fields {
                let expected = &declared_fields[*field];
                let Some(actual) = infer_expression(value, symbols, locals, span, diagnostics)
                else {
                    compatible = false;
                    continue;
                };
                if !types_compatible(expected, &actual) {
                    compatible = false;
                }
            }
            if compatible {
                matches.push(name.clone());
            }
        }
        return match matches.as_slice() {
            [name] => Some(Type::Named(name.clone())),
            [] => {
                diagnostics.push(Diagnostic::error(
                    "SFT047",
                    "object literal does not match any declared Rustify struct",
                    Some(
                        "Use exactly the fields and types of one declared object type.".to_owned(),
                    ),
                    span,
                ));
                None
            }
            _ => {
                diagnostics.push(Diagnostic::error(
                    "SFT048",
                    "object literal matches more than one declared Rustify struct",
                    Some(
                        "Add a distinguishing field or use a named constructor function."
                            .to_owned(),
                    ),
                    span,
                ));
                None
            }
        };
    }
    if expression.parse::<f64>().is_ok() {
        return Some(Type::Number);
    }
    if let Some((callee, args)) = parse_call(expression) {
        if let Some((receiver, "pop")) = split_property_access(callee) {
            let arguments = split_arguments(args);
            if !arguments.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`pop` expects 0 arguments, found {}", arguments.len()),
                    Some("Call pop without arguments.".to_owned()),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            let Type::Array(element_type) = receiver_type else {
                diagnostics.push(Diagnostic::error(
                    "SFT054",
                    format!(
                        "method `pop` is not supported on `{}`",
                        type_name(&receiver_type)
                    ),
                    Some("Call pop on a mutable array binding.".to_owned()),
                    span,
                ));
                return None;
            };
            return Some(Type::Optional(element_type));
        }
        if let Some((receiver, "join")) = split_property_access(callee) {
            let arguments = split_arguments(args);
            if arguments.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`join` expects 1 argument, found {}", arguments.len()),
                    Some("Pass exactly one string separator.".to_owned()),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            if receiver_type != Type::Array(Box::new(Type::String)) {
                diagnostics.push(Diagnostic::error(
                    "SFT054",
                    format!(
                        "method `join` is not supported on `{}`",
                        type_name(&receiver_type)
                    ),
                    Some("Use join on a string[] value.".to_owned()),
                    span,
                ));
                return None;
            }
            let argument_type = infer_expression(arguments[0], symbols, locals, span, diagnostics)?;
            if argument_type != Type::String {
                diagnostics.push(Diagnostic::error(
                    "SFT034",
                    format!(
                        "argument to `join` has type `{}`, expected `string`",
                        type_name(&argument_type)
                    ),
                    Some("Use a string separator.".to_owned()),
                    span,
                ));
            }
            return Some(Type::String);
        }
        if let Some((receiver, "push")) = split_property_access(callee) {
            let arguments = split_arguments(args);
            if arguments.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`push` expects 1 argument, found {}", arguments.len()),
                    Some("Push exactly one value.".to_owned()),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            let Type::Array(element_type) = receiver_type else {
                diagnostics.push(Diagnostic::error(
                    "SFT054",
                    format!(
                        "method `push` is not supported on `{}`",
                        type_name(&receiver_type)
                    ),
                    Some("Call push on a mutable array binding.".to_owned()),
                    span,
                ));
                return None;
            };
            let argument_type = infer_expression(arguments[0], symbols, locals, span, diagnostics)?;
            if !types_compatible(&element_type, &argument_type) {
                diagnostics.push(Diagnostic::error(
                    "SFT034",
                    format!(
                        "argument to `push` has type `{}`, expected `{}`",
                        type_name(&argument_type),
                        type_name(&element_type)
                    ),
                    Some("Push a value matching the array element type.".to_owned()),
                    span,
                ));
            }
            return Some(Type::Void);
        }
        if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "includes" | "startsWith" | "endsWith")
        {
            let arguments = split_arguments(args);
            if arguments.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`{method}` expects 1 argument, found {}", arguments.len()),
                    Some("Pass exactly one search value.".to_owned()),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            let argument_type = infer_expression(arguments[0], symbols, locals, span, diagnostics)?;
            let expected = match (&receiver_type, method) {
                (Type::Array(inner), "includes") => inner.as_ref(),
                (Type::String, _) => &Type::String,
                _ => {
                    diagnostics.push(Diagnostic::error(
                        "SFT054",
                        format!(
                            "method `{method}` is not supported on `{}`",
                            type_name(&receiver_type)
                        ),
                        Some("Use array.includes or a supported string search method.".to_owned()),
                        span,
                    ));
                    return None;
                }
            };
            if !types_compatible(expected, &argument_type) {
                diagnostics.push(Diagnostic::error(
                    "SFT034",
                    format!(
                        "argument to `{method}` has type `{}`, expected `{}`",
                        type_name(&argument_type),
                        type_name(expected)
                    ),
                    Some("Search with a value matching the receiver element type.".to_owned()),
                    span,
                ));
            }
            return Some(Type::Boolean);
        }
        if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "trim" | "toUpperCase" | "toLowerCase")
        {
            let arguments = split_arguments(args);
            if !arguments.is_empty() {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`{method}` expects 0 arguments, found {}", arguments.len()),
                    Some("Call the string transformation without arguments.".to_owned()),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            if receiver_type != Type::String {
                diagnostics.push(Diagnostic::error(
                    "SFT054",
                    format!(
                        "method `{method}` is not supported on `{}`",
                        type_name(&receiver_type)
                    ),
                    Some("Use the transformation on a string value.".to_owned()),
                    span,
                ));
                return None;
            }
            return Some(Type::String);
        }
        if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "isSome" | "isNone" | "isOk" | "isErr" | "unwrapOr")
        {
            let arguments = split_arguments(args);
            let expected_arguments = usize::from(method == "unwrapOr");
            if arguments.len() != expected_arguments {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!(
                        "`{method}` expects {expected_arguments} argument(s), found {}",
                        arguments.len()
                    ),
                    Some(if method == "unwrapOr" {
                        "Pass exactly one fallback value.".to_owned()
                    } else {
                        format!("Call {method} without arguments.")
                    }),
                    span,
                ));
                return None;
            }
            let receiver_type = infer_expression(receiver, symbols, locals, span, diagnostics)?;
            let value_type = match (&receiver_type, method) {
                (Type::Optional(inner), "isSome" | "isNone" | "unwrapOr") => inner,
                (Type::Result(ok, _), "isOk" | "isErr" | "unwrapOr") => ok,
                _ => {
                    diagnostics.push(Diagnostic::error(
                        "SFT054",
                        format!(
                            "method `{method}` is not supported on `{}`",
                            type_name(&receiver_type)
                        ),
                        Some(
                            "Use isSome/isNone on optional values or isOk/isErr on Result values."
                                .to_owned(),
                        ),
                        span,
                    ));
                    return None;
                }
            };
            if method == "unwrapOr" {
                let fallback = infer_expression(arguments[0], symbols, locals, span, diagnostics)?;
                if !types_compatible(value_type, &fallback) {
                    diagnostics.push(Diagnostic::error(
                        "SFT034",
                        format!(
                            "fallback to `unwrapOr` has type `{}`, expected `{}`",
                            type_name(&fallback),
                            type_name(value_type)
                        ),
                        Some("Use a fallback matching the contained value type.".to_owned()),
                        span,
                    ));
                }
                return Some((**value_type).clone());
            }
            return Some(Type::Boolean);
        }
        if matches!(callee, "Ok" | "Err") {
            let arguments = split_arguments(args);
            if arguments.len() != 1 {
                diagnostics.push(Diagnostic::error(
                    "SFT030",
                    format!("`{callee}` expects 1 argument, found {}", arguments.len()),
                    Some("Pass exactly one Result value.".to_owned()),
                    span,
                ));
                return None;
            }
            let value = infer_expression(arguments[0], symbols, locals, span, diagnostics)?;
            return Some(if callee == "Ok" {
                Type::Result(Box::new(value), Box::new(Type::Void))
            } else {
                Type::Result(Box::new(Type::Void), Box::new(value))
            });
        }
        let Some(signature) = symbols.functions.get(callee) else {
            diagnostics.push(Diagnostic::error(
                "SFT031",
                format!("unknown function `{callee}`"),
                Some("Declare the function before calling it.".to_owned()),
                span,
            ));
            return None;
        };
        let arguments = split_arguments(args);
        if arguments.len() != signature.params.len() {
            diagnostics.push(Diagnostic::error(
                "SFT030",
                format!(
                    "`{callee}` expects {} argument(s), found {}",
                    signature.params.len(),
                    arguments.len()
                ),
                Some("Pass the declared number of arguments.".to_owned()),
                span,
            ));
        }
        for (argument, expected) in arguments.iter().zip(&signature.params) {
            if let Some(actual) = infer_expression(argument, symbols, locals, span, diagnostics)
                && !types_compatible(expected, &actual)
            {
                diagnostics.push(Diagnostic::error(
                    "SFT034",
                    format!(
                        "argument to `{callee}` has type `{}`, expected `{}`",
                        type_name(&actual),
                        type_name(expected)
                    ),
                    Some("Pass an argument matching the parameter type.".to_owned()),
                    span,
                ));
            }
        }
        return Some(signature.return_type.clone());
    }
    if let Some((object, property)) = split_property_access(expression) {
        if let Some(variants) = symbols.enums.get(object) {
            if variants.contains(property.trim()) {
                return Some(Type::Named(object.to_owned()));
            }
            diagnostics.push(Diagnostic::error(
                "SFT041",
                format!("unknown enum variant `{property}` on `{object}`"),
                Some("Use a variant declared on the enum.".to_owned()),
                span,
            ));
            return None;
        }
        return match infer_expression(object, symbols, locals, span, diagnostics)? {
            Type::Named(struct_name) => {
                let Some(ty) = symbols
                    .structs
                    .get(&struct_name)
                    .and_then(|fields| fields.get(property))
                else {
                    diagnostics.push(Diagnostic::error(
                        "SFT035",
                        format!("unknown property `{property}` on `{struct_name}`"),
                        Some("Use a field declared on the struct.".to_owned()),
                        span,
                    ));
                    return None;
                };
                Some(ty.clone())
            }
            Type::Array(_) if property == "length" => Some(Type::Number),
            actual => {
                diagnostics.push(Diagnostic::error(
                    "SFT035",
                    format!(
                        "cannot access property `{property}` on `{}`",
                        type_name(&actual)
                    ),
                    Some("Use a declared struct field or array.length.".to_owned()),
                    span,
                ));
                None
            }
        };
    }
    if let Some(ty) = locals.get(expression) {
        return Some(ty.clone());
    }
    diagnostics.push(Diagnostic::error(
        "SFT031",
        format!("unknown identifier or unsupported expression `{expression}`"),
        Some("Use a declared identifier or a supported expression.".to_owned()),
        span,
    ));
    None
}

fn infer_binary_expression(
    expression: &str,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Type> {
    if let Some((left, operator, right)) = split_binary_parts(expression) {
        let left = infer_expression(left, symbols, locals, span, diagnostics)?;
        let right = infer_expression(right, symbols, locals, span, diagnostics)?;
        if !types_compatible(&left, &right) {
            diagnostics.push(Diagnostic::error(
                "SFT036",
                format!(
                    "operator `{operator}` cannot combine `{}` and `{}`",
                    type_name(&left),
                    type_name(&right)
                ),
                Some("Use operands with compatible types.".to_owned()),
                span,
            ));
            return None;
        }
        let valid = match operator {
            "&&" | "||" => left == Type::Boolean,
            "+" => matches!(left, Type::Number | Type::String),
            "-" | "*" | "/" | "%" | ">" | "<" | ">=" | "<=" => left == Type::Number,
            "===" | "!==" | "==" | "!=" => true,
            _ => false,
        };
        if !valid {
            diagnostics.push(Diagnostic::error(
                "SFT036",
                format!(
                    "operator `{operator}` is not supported for `{}`",
                    type_name(&left)
                ),
                Some("Use an operator supported by the operand type.".to_owned()),
                span,
            ));
            return None;
        }
        return Some(
            if matches!(
                operator,
                "&&" | "||" | "===" | "!==" | "==" | "!=" | ">" | "<" | ">=" | "<="
            ) {
                Type::Boolean
            } else {
                left
            },
        );
    }
    None
}

fn validate_type(ty: &Type, span: Span, symbols: &SymbolTable, diagnostics: &mut Vec<Diagnostic>) {
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
        Type::Named(name)
            if !symbols.structs.contains_key(name) && !symbols.enums.contains_key(name) =>
        {
            diagnostics.push(Diagnostic::error(
                "SFT021",
                format!("unknown type `{name}`"),
                Some("Declare the type before using it.".to_owned()),
                span,
            ));
        }
        Type::Array(inner) | Type::Optional(inner) | Type::Promise(inner) => {
            validate_type(inner, span, symbols, diagnostics)
        }
        Type::Result(ok, error) => {
            validate_type(ok, span, symbols, diagnostics);
            validate_type(error, span, symbols, diagnostics);
        }
        _ => {}
    }
}

fn rust_name_collision(first: &str, second: &str, rust_name: &str, span: Span) -> Diagnostic {
    Diagnostic::error(
        "SFT064",
        format!("`{first}` and `{second}` both compile to Rust identifier `{rust_name}`"),
        Some("Rename one symbol so its generated Rust identifier is unique.".to_owned()),
        span,
    )
}

fn rust_identifier_key(name: &str) -> String {
    let mut output = String::new();
    for (index, character) in name.chars().enumerate() {
        if character.is_ascii_uppercase() {
            if index > 0 && !output.ends_with('_') {
                output.push('_');
            }
            output.push(character.to_ascii_lowercase());
        } else if character == '$' {
            output.push_str("_dollar_");
        } else {
            output.push(character);
        }
    }
    output
}

fn rust_type_identifier_key(name: &str) -> String {
    let mut output = String::new();
    let mut capitalize = true;
    for character in name.chars() {
        if character == '$' {
            output.push_str("Dollar");
            capitalize = true;
        } else if character == '_' {
            capitalize = true;
        } else if capitalize {
            output.extend(character.to_uppercase());
            capitalize = false;
        } else {
            output.push(character);
        }
    }
    if output == "Self" {
        "RustifySelf".to_owned()
    } else {
        output
    }
}

fn split_statements(body: &str) -> Vec<&str> {
    let mut statements = Vec::new();
    let mut start = 0;
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
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
            '{' => depth += 1,
            '}' => depth -= 1,
            ';' | '\n' if depth == 0 => {
                if !body[start..index].trim().is_empty() {
                    statements.push(body[start..index].trim());
                }
                start = index + character.len_utf8();
            }
            _ => {}
        }
    }
    if !body[start..].trim().is_empty() {
        statements.push(body[start..].trim());
    }
    statements
        .into_iter()
        .filter(|statement| {
            let statement = statement.trim();
            !statement.starts_with("//") && !statement.starts_with("/*")
        })
        .collect()
}

fn body_definitely_returns(body: &str) -> bool {
    split_statements(body)
        .into_iter()
        .any(statement_definitely_returns)
}

fn statement_definitely_returns(statement: &str) -> bool {
    let statement = statement.trim();
    if statement == "return" || statement.starts_with("return ") {
        return true;
    }
    let Some(rest) = statement.strip_prefix("if ") else {
        return false;
    };
    let Some(block_start) = rest.find('{') else {
        return false;
    };
    let Some(block_end) = matching_brace(rest, block_start) else {
        return false;
    };
    if !body_definitely_returns(&rest[block_start + 1..block_end]) {
        return false;
    }
    let Some(else_body) = rest[block_end + 1..].trim().strip_prefix("else") else {
        return false;
    };
    let else_body = else_body.trim();
    if else_body.starts_with("if ") {
        statement_definitely_returns(else_body)
    } else if else_body.starts_with('{') {
        matching_brace(else_body, 0).is_some_and(|end| body_definitely_returns(&else_body[1..end]))
    } else {
        false
    }
}

fn strip_parentheses(value: &str) -> &str {
    value
        .trim()
        .strip_prefix('(')
        .and_then(|value| value.strip_suffix(')'))
        .unwrap_or(value.trim())
}

fn strip_expression_parentheses(mut value: &str) -> &str {
    loop {
        value = value.trim();
        if !value.starts_with('(')
            || matching_parenthesis(value, 0) != Some(value.len().saturating_sub(1))
        {
            return value;
        }
        value = &value[1..value.len() - 1];
    }
}

fn matching_parenthesis(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    for (offset, character) in source[start..].char_indices() {
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
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn matching_brace(source: &str, start: usize) -> Option<usize> {
    let mut depth = 0;
    for (offset, character) in source[start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(start + offset);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_assignment(statement: &str) -> Option<(&str, &str)> {
    let (left, right) = statement.split_once('=')?;
    (!left.ends_with(['!', '=', '>', '<']) && !right.starts_with('='))
        .then(|| (left.trim(), right.trim()))
}

fn parse_call(expression: &str) -> Option<(&str, &str)> {
    if !expression.ends_with(')') {
        return None;
    }
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    let mut open = None;
    for (index, character) in expression.char_indices() {
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
            '(' if depth == 0 => {
                open = Some(index);
                depth = 1;
            }
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }
    let open = open?;
    (depth == 0).then(|| {
        (
            expression[..open].trim(),
            expression[open + 1..expression.len() - 1].trim(),
        )
    })
}

fn array_mutation_receiver(expression: &str) -> Option<(&str, &str)> {
    let (callee, _) = parse_call(expression)?;
    let (receiver, method) = split_property_access(callee)?;
    matches!(method, "push" | "pop").then_some((receiver, method))
}

fn validate_array_mutation(
    expression: &str,
    locals: &HashMap<String, Type>,
    mutable_locals: &HashSet<String>,
    span: Span,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for (receiver, method) in array_mutation_calls(expression) {
        if !locals.contains_key(receiver) {
            diagnostics.push(Diagnostic::error(
                "SFT056",
                format!("`array.{method}` requires a local array binding"),
                Some("Assign the array to a local `let` binding before mutating it.".to_owned()),
                span,
            ));
        } else if !mutable_locals.contains(receiver) {
            diagnostics.push(Diagnostic::error(
                "SFT055",
                format!("cannot mutate immutable binding `{receiver}`"),
                Some(format!(
                    "Declare the array with `let` before calling {method}."
                )),
                span,
            ));
        }
    }
}

fn reject_push_value(expression: &str, span: Span, diagnostics: &mut Vec<Diagnostic>) {
    if array_mutation_calls(expression)
        .iter()
        .any(|(_, method)| *method == "push")
    {
        diagnostics.push(Diagnostic::error(
            "SFT057",
            "`array.push` cannot be used as a value in native mode",
            Some("Call push as a standalone statement; read array.length separately.".to_owned()),
            span,
        ));
    }
}

fn array_mutation_calls(expression: &str) -> Vec<(&str, &'static str)> {
    let mut calls = Vec::new();
    for method in ["push", "pop"] {
        let needle = format!(".{method}(");
        for (dot, _) in expression.match_indices(&needle) {
            let start = expression[..dot]
                .char_indices()
                .rev()
                .find(|(_, character)| !is_member_expression_character(*character))
                .map_or(0, |(index, character)| index + character.len_utf8());
            let receiver = expression[start..dot].trim();
            if !receiver.is_empty() {
                calls.push((receiver, method));
            }
        }
    }
    calls
}

fn is_member_expression_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '$' | '.')
}

fn split_property_access(expression: &str) -> Option<(&str, &str)> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    let mut dot = None;
    for (index, character) in expression.char_indices() {
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
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '.' if depth == 0 => dot = Some(index),
            _ => {}
        }
    }
    let dot = dot?;
    let object = expression[..dot].trim();
    let property = expression[dot + 1..].trim();
    (!object.is_empty() && !property.is_empty()).then_some((object, property))
}

fn split_index_access(expression: &str) -> Option<(&str, &str)> {
    let expression = expression.trim();
    if !expression.ends_with(']') {
        return None;
    }
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut matching_open = None;
    for (index, character) in expression.char_indices() {
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
            '(' | '[' | '{' => stack.push((character, index)),
            ')' | ']' | '}' => {
                let expected = match character {
                    ')' => '(',
                    ']' => '[',
                    '}' => '{',
                    _ => unreachable!(),
                };
                let (open, open_index) = stack.pop()?;
                if open != expected {
                    return None;
                }
                if character == ']' && index + character.len_utf8() == expression.len() {
                    matching_open = Some(open_index);
                }
            }
            _ => {}
        }
    }
    if quote.is_some() || !stack.is_empty() {
        return None;
    }
    let open = matching_open?;
    let array = expression[..open].trim();
    let index = expression[open + 1..expression.len() - 1].trim();
    (!array.is_empty() && !index.is_empty()).then_some((array, index))
}

fn split_arguments(args: &str) -> Vec<&str> {
    if args.trim().is_empty() {
        Vec::new()
    } else {
        split_top_level_values(args, ',')
    }
}

fn split_conditional_expression(expression: &str) -> Option<(&str, &str, &str)> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    let mut question = None;
    let mut nested_questions = 0_u32;
    for (index, character) in expression.char_indices() {
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
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            '?' if depth == 0 && question.is_none() => question = Some(index),
            '?' if depth == 0 => nested_questions += 1,
            ':' if depth == 0 && question.is_some() && nested_questions == 0 => {
                let question = question?;
                let condition = expression[..question].trim();
                let then_value = expression[question + 1..index].trim();
                let else_value = expression[index + 1..].trim();
                return (!condition.is_empty() && !then_value.is_empty() && !else_value.is_empty())
                    .then_some((condition, then_value, else_value));
            }
            ':' if depth == 0 && question.is_some() => nested_questions -= 1,
            _ => {}
        }
    }
    None
}

fn parse_object_literal(expression: &str) -> Option<Vec<(&str, &str)>> {
    let inner = expression.strip_prefix('{')?.strip_suffix('}')?;
    if inner.trim().is_empty() {
        return Some(Vec::new());
    }
    split_top_level_values(inner, ',')
        .into_iter()
        .map(|field| {
            let (name, value) = field.split_once(':')?;
            Some((name.trim(), value.trim()))
        })
        .collect()
}

fn split_top_level_values(input: &str, separator: char) -> Vec<&str> {
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
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            value if value == separator && depth == 0 => {
                parts.push(input[start..index].trim());
                start = index + value.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(input[start..].trim());
    parts
}

fn split_binary_parts(expression: &str) -> Option<(&str, &'static str, &str)> {
    for operators in [
        &["||"][..],
        &["&&"][..],
        &["===", "!==", "==", "!="][..],
        &[">=", "<=", ">", "<"][..],
        &["+", "-"][..],
        &["*", "/", "%"][..],
    ] {
        if let Some(parts) = split_top_level_operator(expression, operators) {
            return Some(parts);
        }
    }
    None
}

fn split_top_level_operator<'a>(
    expression: &'a str,
    operators: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    let mut depth = 0_i32;
    let mut quote = None;
    let mut escaped = false;
    let mut found = None;
    for (index, character) in expression.char_indices() {
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
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => depth -= 1,
            _ if depth == 0 => {
                if expression[..index].ends_with(['=', '!', '<', '>', '&', '|']) {
                    continue;
                }
                for operator in operators {
                    if expression[index..].starts_with(operator)
                        && !expression[..index].trim().is_empty()
                        && !expression[index + operator.len()..].trim().is_empty()
                    {
                        found = Some((
                            expression[..index].trim(),
                            *operator,
                            expression[index + operator.len()..].trim(),
                        ));
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    found
}

fn types_compatible(expected: &Type, actual: &Type) -> bool {
    expected == actual
        || matches!(
            (expected, actual),
            (Type::Optional(_), Type::Optional(actual)) if **actual == Type::Void
        )
        || matches!(
            (expected, actual),
            (Type::Optional(expected), actual) if types_compatible(expected, actual)
        )
        || matches!(
            (expected, actual),
            (Type::Result(expected_ok, expected_error), Type::Result(actual_ok, actual_error))
                if result_component_compatible(expected_ok, actual_ok)
                    && result_component_compatible(expected_error, actual_error)
        )
        || matches!(
            (expected, actual),
            (Type::Promise(expected), Type::Promise(actual)) if types_compatible(expected, actual)
        )
        || matches!(
            (expected, actual),
            (Type::Array(expected), Type::Array(actual))
                if **actual == Type::Void || types_compatible(expected, actual)
        )
}

fn type_contains_promise(ty: &Type) -> bool {
    match ty {
        Type::Promise(_) => true,
        Type::Array(inner) | Type::Optional(inner) => type_contains_promise(inner),
        Type::Result(ok, error) => type_contains_promise(ok) || type_contains_promise(error),
        _ => false,
    }
}

fn is_direct_promise(ty: &Type) -> bool {
    matches!(ty, Type::Promise(inner) if !type_contains_promise(inner))
}

fn type_is_debuggable(ty: &Type) -> bool {
    match ty {
        Type::Promise(_) => false,
        Type::Array(inner) | Type::Optional(inner) => type_is_debuggable(inner),
        Type::Result(ok, error) => type_is_debuggable(ok) && type_is_debuggable(error),
        _ => true,
    }
}

fn compatible_conditional_type(then_type: &Type, else_type: &Type) -> Option<Type> {
    if then_type == else_type {
        Some(then_type.clone())
    } else if matches!(else_type, Type::Optional(inner) if **inner == Type::Void) {
        Some(nullable_conditional_type(then_type))
    } else if matches!(then_type, Type::Optional(inner) if **inner == Type::Void) {
        Some(nullable_conditional_type(else_type))
    } else if types_compatible(then_type, else_type) {
        Some(then_type.clone())
    } else if types_compatible(else_type, then_type) {
        Some(else_type.clone())
    } else {
        None
    }
}

fn nullable_conditional_type(ty: &Type) -> Type {
    if matches!(ty, Type::Optional(_)) {
        ty.clone()
    } else {
        Type::Optional(Box::new(ty.clone()))
    }
}

fn result_component_compatible(expected: &Type, actual: &Type) -> bool {
    actual == &Type::Void || types_compatible(expected, actual)
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::String => "string".to_owned(),
        Type::Number => "number".to_owned(),
        Type::Boolean => "boolean".to_owned(),
        Type::Void => "void".to_owned(),
        Type::JsonValue => "JsonValue".to_owned(),
        Type::Named(name) | Type::Unsupported(name) => name.clone(),
        Type::Array(inner) => format!("{}[]", type_name(inner)),
        Type::Optional(inner) => format!("{} | null", type_name(inner)),
        Type::Result(ok, error) => format!("Result<{}, {}>", type_name(ok), type_name(error)),
        Type::Promise(inner) => format!("Promise<{}>", type_name(inner)),
    }
}

fn lower(program: &Program, symbols: &SymbolTable) -> ir::Program {
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
            .map(|function| lower_function(function, symbols))
            .collect(),
    }
}

fn lower_function(function: &rustify_parser::FunctionDecl, symbols: &SymbolTable) -> ir::Function {
    let mut locals: HashMap<String, Type> = function
        .params
        .iter()
        .map(|parameter| {
            (
                parameter.name.clone(),
                parameter.ty.clone().expect("validated parameter type"),
            )
        })
        .collect();
    ir::Function {
        name: function.name.clone(),
        is_async: function.is_async,
        params: function
            .params
            .iter()
            .map(|parameter| ir::Parameter {
                name: parameter.name.clone(),
                ty: lower_type(parameter.ty.as_ref().expect("validated parameter type")),
            })
            .collect(),
        return_type: lower_type(
            function_return_value_type(function).expect("validated return type"),
        ),
        body: {
            let mut body = lower_body(&function.body, symbols, &mut locals);
            coerce_returns(
                &mut body,
                &lower_type(function_return_value_type(function).expect("validated return type")),
            );
            body
        },
    }
}

fn lower_body(
    body: &str,
    symbols: &SymbolTable,
    locals: &mut HashMap<String, Type>,
) -> Vec<ir::Statement> {
    split_statements(body)
        .into_iter()
        .map(|statement| {
            lower_statement(statement.trim(), symbols, locals)
                .expect("validated statement must lower to typed IR")
        })
        .collect()
}

fn lower_statement(
    statement: &str,
    symbols: &SymbolTable,
    locals: &mut HashMap<String, Type>,
) -> Option<ir::Statement> {
    if let Some(rest) = statement.strip_prefix("const ") {
        return lower_variable(rest, false, symbols, locals);
    }
    if let Some(rest) = statement.strip_prefix("let ") {
        return lower_variable(rest, true, symbols, locals);
    }
    if let Some(expression) = statement.strip_prefix("return ") {
        return Some(ir::Statement::Return(lower_expression(
            expression, symbols, locals,
        )?));
    }
    if statement == "return" {
        return Some(ir::Statement::ReturnVoid);
    }
    if statement == "break" {
        return Some(ir::Statement::Break);
    }
    if statement == "continue" {
        return Some(ir::Statement::Continue);
    }
    if let Some(argument) = statement
        .strip_prefix("console.log(")
        .and_then(|value| value.strip_suffix(')'))
    {
        return Some(ir::Statement::ConsoleLog(
            split_arguments(argument)
                .into_iter()
                .map(|argument| lower_expression(argument, symbols, locals))
                .collect::<Option<Vec<_>>>()?,
        ));
    }
    if statement.starts_with("if ")
        || statement.starts_with("while ")
        || statement.starts_with("for ")
    {
        return lower_control_flow(statement, symbols, locals);
    }
    if let Some((name, expression)) = split_assignment(statement) {
        let mut value = lower_expression(expression, symbols, locals)?;
        if let Some(expected) = locals.get(name) {
            value = coerce_expression(value, &lower_type(expected));
        }
        return Some(ir::Statement::Assignment {
            name: name.to_owned(),
            value,
        });
    }
    Some(ir::Statement::Expression(lower_expression(
        statement, symbols, locals,
    )?))
}

fn lower_variable(
    declaration: &str,
    mutable: bool,
    symbols: &SymbolTable,
    locals: &mut HashMap<String, Type>,
) -> Option<ir::Statement> {
    let (left, expression) = declaration.split_once('=')?;
    let (name, annotation) = left
        .split_once(':')
        .map(|(name, ty)| (name.trim(), Some(rustify_parser::parse_type(ty.trim()))))
        .unwrap_or((left.trim(), None));
    let mut value = lower_expression(expression, symbols, locals)?;
    let ty = annotation.unwrap_or_else(|| upper_type(&value.ty));
    value = coerce_expression(value, &lower_type(&ty));
    locals.insert(name.to_owned(), ty.clone());
    Some(ir::Statement::Variable {
        name: name.to_owned(),
        mutable,
        ty: lower_type(&ty),
        value,
    })
}

fn lower_control_flow(
    statement: &str,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
) -> Option<ir::Statement> {
    let block_start = statement.find('{')?;
    let block_end = matching_brace(statement, block_start)?;
    let header = statement[..block_start].trim();
    let body = &statement[block_start + 1..block_end];
    let mut nested_locals = locals.clone();
    if let Some(condition) = header.strip_prefix("if ") {
        let condition = lower_expression(strip_parentheses(condition), symbols, locals)?;
        let then_body = lower_body(body, symbols, &mut nested_locals);
        let remainder = statement[block_end + 1..].trim();
        let else_body = if let Some(value) = remainder.strip_prefix("else") {
            let value = value.trim();
            if value.starts_with("if ") {
                lower_control_flow(value, symbols, locals)
                    .map(|statement| vec![statement])
                    .unwrap_or_default()
            } else {
                let end = matching_brace(value, 0)?;
                lower_body(&value[1..end], symbols, &mut locals.clone())
            }
        } else {
            Vec::new()
        };
        return Some(ir::Statement::If {
            condition,
            then_body,
            else_body,
        });
    }
    if let Some(condition) = header.strip_prefix("while ") {
        return Some(ir::Statement::While {
            condition: lower_expression(strip_parentheses(condition), symbols, locals)?,
            body: lower_body(body, symbols, &mut nested_locals),
        });
    }
    let iteration = strip_parentheses(header.strip_prefix("for ")?)
        .trim_start_matches("const ")
        .trim_start_matches("let ");
    let (binding, iterable) = iteration.split_once(" of ")?;
    let iterable = lower_expression(iterable, symbols, locals)?;
    if let ir::Type::Vec(inner) = &iterable.ty {
        nested_locals.insert(binding.trim().to_owned(), upper_type(inner));
    }
    Some(ir::Statement::ForOf {
        binding: binding.trim().to_owned(),
        iterable,
        body: lower_body(body, symbols, &mut nested_locals),
    })
}

fn lower_expression(
    expression: &str,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
) -> Option<ir::Expression> {
    let expression = expression.trim().trim_end_matches(';').trim();
    let expression = strip_expression_parentheses(expression);
    let mut ignored = Vec::new();
    let ty = lower_type(&infer_expression(
        expression,
        symbols,
        locals,
        Span { start: 0, end: 0 },
        &mut ignored,
    )?);
    let kind = if expression.starts_with('`') && expression.ends_with('`') {
        lower_template(expression, symbols, locals)?
    } else if let Some(inner) = expression.strip_prefix("await ") {
        ir::ExpressionKind::Await(Box::new(lower_expression(inner, symbols, locals)?))
    } else if let Some((condition, then_value, else_value)) =
        split_conditional_expression(expression)
    {
        ir::ExpressionKind::Conditional {
            condition: Box::new(lower_expression(condition, symbols, locals)?),
            then_value: Box::new(coerce_expression(
                lower_expression(then_value, symbols, locals)?,
                &ty,
            )),
            else_value: Box::new(coerce_expression(
                lower_expression(else_value, symbols, locals)?,
                &ty,
            )),
        }
    } else if (expression.starts_with('"') && expression.ends_with('"'))
        || (expression.starts_with('\'') && expression.ends_with('\''))
    {
        ir::ExpressionKind::String(expression[1..expression.len() - 1].to_owned())
    } else if matches!(expression, "true" | "false") {
        ir::ExpressionKind::Boolean(expression == "true")
    } else if matches!(expression, "null" | "undefined") {
        ir::ExpressionKind::Null
    } else if let Ok(value) = expression.parse::<f64>() {
        ir::ExpressionKind::Number(value)
    } else if expression.starts_with('[') && expression.ends_with(']') {
        ir::ExpressionKind::Array(
            split_arguments(&expression[1..expression.len() - 1])
                .into_iter()
                .map(|value| lower_expression(value, symbols, locals))
                .collect::<Option<Vec<_>>>()?,
        )
    } else if let Some(fields) = parse_object_literal(expression) {
        let ir::Type::Named(name) = &ty else {
            return None;
        };
        let declared = symbols.structs.get(name)?;
        let mut lowered = Vec::new();
        let mut declared: Vec<_> = declared.iter().collect();
        declared.sort_by_key(|(field, _)| *field);
        for (field, expected) in declared {
            let value =
                if let Some((_, value)) = fields.iter().find(|(provided, _)| provided == field) {
                    coerce_expression(
                        lower_expression(value, symbols, locals)?,
                        &lower_type(expected),
                    )
                } else if matches!(expected, Type::Optional(_)) {
                    ir::Expression {
                        ty: lower_type(expected),
                        kind: ir::ExpressionKind::Null,
                    }
                } else {
                    return None;
                };
            lowered.push((field.clone(), value));
        }
        ir::ExpressionKind::StructLiteral {
            name: name.clone(),
            fields: lowered,
        }
    } else if let Some((left, operator, right)) = split_binary(expression) {
        ir::ExpressionKind::Binary {
            left: Box::new(lower_expression(left, symbols, locals)?),
            operator,
            right: Box::new(lower_expression(right, symbols, locals)?),
        }
    } else if let Some(inner) = expression.strip_prefix('!') {
        ir::ExpressionKind::Unary {
            operator: ir::UnaryOperator::Not,
            value: Box::new(lower_expression(inner, symbols, locals)?),
        }
    } else if let Some(inner) = expression.strip_prefix('-')
        && expression.parse::<f64>().is_err()
    {
        ir::ExpressionKind::Unary {
            operator: ir::UnaryOperator::Negate,
            value: Box::new(lower_expression(inner, symbols, locals)?),
        }
    } else if let Some((array, index)) = split_index_access(expression) {
        ir::ExpressionKind::ArrayGet {
            array: Box::new(lower_expression(array, symbols, locals)?),
            index: Box::new(lower_expression(index, symbols, locals)?),
        }
    } else if let Some((callee, arguments)) = parse_call(expression) {
        if let Some((receiver, "pop")) = split_property_access(callee) {
            ir::ExpressionKind::ArrayPop(Box::new(lower_expression(receiver, symbols, locals)?))
        } else if let Some((receiver, "join")) = split_property_access(callee) {
            ir::ExpressionKind::ArrayJoin {
                array: Box::new(lower_expression(receiver, symbols, locals)?),
                separator: Box::new(lower_expression(
                    split_arguments(arguments).into_iter().next()?,
                    symbols,
                    locals,
                )?),
            }
        } else if let Some((receiver, "push")) = split_property_access(callee) {
            ir::ExpressionKind::ArrayPush {
                array: Box::new(lower_expression(receiver, symbols, locals)?),
                value: Box::new(lower_expression(
                    split_arguments(arguments).into_iter().next()?,
                    symbols,
                    locals,
                )?),
            }
        } else if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "includes" | "startsWith" | "endsWith")
        {
            let argument = split_arguments(arguments).into_iter().next()?;
            let receiver = lower_expression(receiver, symbols, locals)?;
            let argument = lower_expression(argument, symbols, locals)?;
            match (&receiver.ty, method) {
                (ir::Type::Vec(_), "includes") => ir::ExpressionKind::ArrayIncludes {
                    array: Box::new(receiver),
                    value: Box::new(argument),
                },
                (ir::Type::String, _) => ir::ExpressionKind::StringMethod {
                    receiver: Box::new(receiver),
                    method: match method {
                        "includes" => ir::StringMethod::Includes,
                        "startsWith" => ir::StringMethod::StartsWith,
                        "endsWith" => ir::StringMethod::EndsWith,
                        _ => return None,
                    },
                    argument: Box::new(argument),
                },
                _ => return None,
            }
        } else if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "trim" | "toUpperCase" | "toLowerCase")
        {
            ir::ExpressionKind::StringTransform {
                receiver: Box::new(lower_expression(receiver, symbols, locals)?),
                transform: match method {
                    "trim" => ir::StringTransform::Trim,
                    "toUpperCase" => ir::StringTransform::ToUpperCase,
                    "toLowerCase" => ir::StringTransform::ToLowerCase,
                    _ => return None,
                },
            }
        } else if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "isSome" | "isNone")
        {
            ir::ExpressionKind::OptionCheck {
                value: Box::new(lower_expression(receiver, symbols, locals)?),
                is_some: method == "isSome",
            }
        } else if let Some((receiver, method)) = split_property_access(callee)
            && matches!(method, "isOk" | "isErr")
        {
            ir::ExpressionKind::ResultCheck {
                value: Box::new(lower_expression(receiver, symbols, locals)?),
                is_ok: method == "isOk",
            }
        } else if let Some((receiver, "unwrapOr")) = split_property_access(callee)
            && matches!(
                infer_expression(
                    receiver,
                    symbols,
                    locals,
                    Span { start: 0, end: 0 },
                    &mut Vec::new(),
                ),
                Some(Type::Optional(_))
            )
        {
            ir::ExpressionKind::OptionUnwrapOr {
                value: Box::new(lower_expression(receiver, symbols, locals)?),
                fallback: Box::new(lower_expression(
                    split_arguments(arguments).into_iter().next()?,
                    symbols,
                    locals,
                )?),
            }
        } else if let Some((receiver, "unwrapOr")) = split_property_access(callee) {
            ir::ExpressionKind::ResultUnwrapOr {
                value: Box::new(lower_expression(receiver, symbols, locals)?),
                fallback: Box::new(lower_expression(
                    split_arguments(arguments).into_iter().next()?,
                    symbols,
                    locals,
                )?),
            }
        } else {
            let expected = symbols
                .functions
                .get(callee)
                .map(|signature| &signature.params);
            ir::ExpressionKind::Call {
                callee: callee.to_owned(),
                arguments: split_arguments(arguments)
                    .into_iter()
                    .enumerate()
                    .map(|(index, argument)| {
                        let value = lower_expression(argument, symbols, locals)?;
                        Some(match expected.and_then(|params| params.get(index)) {
                            Some(expected) => coerce_expression(value, &lower_type(expected)),
                            None => value,
                        })
                    })
                    .collect::<Option<Vec<_>>>()?,
            }
        }
    } else if let Some((object, property)) = split_property_access(expression) {
        if symbols.enums.contains_key(object) {
            ir::ExpressionKind::EnumVariant {
                enumeration: object.to_owned(),
                variant: property.to_owned(),
            }
        } else if matches!(
            infer_expression(
                object,
                symbols,
                locals,
                Span { start: 0, end: 0 },
                &mut Vec::new(),
            ),
            Some(Type::Array(_))
        ) && property == "length"
        {
            ir::ExpressionKind::ArrayLength(Box::new(lower_expression(object, symbols, locals)?))
        } else {
            ir::ExpressionKind::Property {
                object: Box::new(lower_expression(object, symbols, locals)?),
                property: property.to_owned(),
            }
        }
    } else {
        ir::ExpressionKind::Identifier(expression.to_owned())
    };
    Some(ir::Expression { ty, kind })
}

fn lower_template(
    template: &str,
    symbols: &SymbolTable,
    locals: &HashMap<String, Type>,
) -> Option<ir::ExpressionKind> {
    let mut parts = Vec::new();
    let mut expressions = Vec::new();
    let mut remainder = &template[1..template.len() - 1];
    while let Some(start) = remainder.find("${") {
        parts.push(remainder[..start].to_owned());
        let after_start = &remainder[start + 2..];
        let end = after_start.find('}')?;
        expressions.push(lower_expression(&after_start[..end], symbols, locals)?);
        remainder = &after_start[end + 1..];
    }
    parts.push(remainder.to_owned());
    Some(ir::ExpressionKind::Template { parts, expressions })
}

fn coerce_expression(expression: ir::Expression, expected: &ir::Type) -> ir::Expression {
    if &expression.ty == expected {
        return expression;
    }
    if let ir::ExpressionKind::Conditional {
        condition,
        then_value,
        else_value,
    } = expression.kind
    {
        return ir::Expression {
            ty: expected.clone(),
            kind: ir::ExpressionKind::Conditional {
                condition,
                then_value: Box::new(coerce_expression(*then_value, expected)),
                else_value: Box::new(coerce_expression(*else_value, expected)),
            },
        };
    }
    if let ir::Type::Option(inner) = expected {
        if matches!(expression.kind, ir::ExpressionKind::Null) {
            return ir::Expression {
                ty: expected.clone(),
                kind: ir::ExpressionKind::Null,
            };
        }
        if expression.ty == **inner {
            return ir::Expression {
                ty: expected.clone(),
                kind: ir::ExpressionKind::Some(Box::new(expression)),
            };
        }
    }
    if matches!(
        (&expression.ty, expected),
        (ir::Type::Result(_, _), ir::Type::Result(_, _))
    ) {
        return ir::Expression {
            ty: expected.clone(),
            kind: expression.kind,
        };
    }
    if matches!(
        (&expression.ty, expected, &expression.kind),
        (ir::Type::Vec(_), ir::Type::Vec(_), ir::ExpressionKind::Array(values)) if values.is_empty()
    ) {
        return ir::Expression {
            ty: expected.clone(),
            kind: expression.kind,
        };
    }
    expression
}

fn coerce_returns(statements: &mut [ir::Statement], expected: &ir::Type) {
    for statement in statements {
        match statement {
            ir::Statement::Return(value) => {
                *value = coerce_expression(value.clone(), expected);
            }
            ir::Statement::If {
                then_body,
                else_body,
                ..
            } => {
                coerce_returns(then_body, expected);
                coerce_returns(else_body, expected);
            }
            ir::Statement::While { body, .. } | ir::Statement::ForOf { body, .. } => {
                coerce_returns(body, expected);
            }
            _ => {}
        }
    }
}

fn split_binary(expression: &str) -> Option<(&str, ir::BinaryOperator, &str)> {
    let (left, token, right) = split_binary_parts(expression)?;
    let operator = match token {
        "||" => ir::BinaryOperator::Or,
        "&&" => ir::BinaryOperator::And,
        "===" | "==" => ir::BinaryOperator::Equal,
        "!==" | "!=" => ir::BinaryOperator::NotEqual,
        ">=" => ir::BinaryOperator::GreaterEqual,
        "<=" => ir::BinaryOperator::LessEqual,
        ">" => ir::BinaryOperator::Greater,
        "<" => ir::BinaryOperator::Less,
        "+" => ir::BinaryOperator::Add,
        "-" => ir::BinaryOperator::Subtract,
        "*" => ir::BinaryOperator::Multiply,
        "/" => ir::BinaryOperator::Divide,
        "%" => ir::BinaryOperator::Remainder,
        _ => return None,
    };
    Some((left, operator, right))
}

fn lower_type(ty: &Type) -> ir::Type {
    match ty {
        Type::String => ir::Type::String,
        Type::Number => ir::Type::F64,
        Type::Boolean => ir::Type::Bool,
        Type::Void => ir::Type::Unit,
        Type::JsonValue => ir::Type::JsonValue,
        Type::Named(name) => ir::Type::Named(name.clone()),
        Type::Array(inner) => ir::Type::Vec(Box::new(lower_type(inner))),
        Type::Optional(inner) => ir::Type::Option(Box::new(lower_type(inner))),
        Type::Result(ok, error) => {
            ir::Type::Result(Box::new(lower_type(ok)), Box::new(lower_type(error)))
        }
        Type::Promise(inner) => ir::Type::Promise(Box::new(lower_type(inner))),
        Type::Unsupported(name) => panic!("unsupported type `{name}` passed validation"),
    }
}

fn upper_type(ty: &ir::Type) -> Type {
    match ty {
        ir::Type::String => Type::String,
        ir::Type::F64 => Type::Number,
        ir::Type::Bool => Type::Boolean,
        ir::Type::Unit => Type::Void,
        ir::Type::JsonValue => Type::JsonValue,
        ir::Type::Named(name) => Type::Named(name.clone()),
        ir::Type::Vec(inner) => Type::Array(Box::new(upper_type(inner))),
        ir::Type::Option(inner) => Type::Optional(Box::new(upper_type(inner))),
        ir::Type::Result(ok, error) => {
            Type::Result(Box::new(upper_type(ok)), Box::new(upper_type(error)))
        }
        ir::Type::Promise(inner) => Type::Promise(Box::new(upper_type(inner))),
    }
}

fn function_return_value_type(function: &rustify_parser::FunctionDecl) -> Option<&Type> {
    match function.return_type.as_ref()? {
        Type::Promise(inner) if function.is_async => Some(inner),
        ty => Some(ty),
    }
}

fn contains_await(body: &str) -> bool {
    body.split(|character: char| !(character.is_alphanumeric() || character == '_'))
        .any(|word| word == "await")
}

fn identifier_reference_count(source: &str, name: &str) -> usize {
    let searchable = mask_strings_and_comments(source);
    searchable
        .match_indices(name)
        .filter(|(start, _)| {
            let before = searchable[..*start].chars().next_back();
            let end = start + name.len();
            let after = searchable[end..].chars().next();
            !before.is_some_and(|character| character.is_alphanumeric() || character == '_')
                && !after.is_some_and(|character| character.is_alphanumeric() || character == '_')
                && !searchable[..*start].trim_end().ends_with('.')
                && !searchable[end..].trim_start().starts_with(':')
        })
        .count()
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
