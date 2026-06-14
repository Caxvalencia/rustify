use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Type {
    String,
    F64,
    Bool,
    Unit,
    JsonValue,
    Named(String),
    Vec(Box<Type>),
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    Promise(Box<Type>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Struct {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Enum {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Expression {
    pub ty: Type,
    pub kind: ExpressionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExpressionKind {
    String(String),
    Number(f64),
    Boolean(bool),
    Null,
    Some(Box<Expression>),
    Await(Box<Expression>),
    Identifier(String),
    Array(Vec<Expression>),
    StructLiteral {
        name: String,
        fields: Vec<(String, Expression)>,
    },
    Call {
        callee: String,
        arguments: Vec<Expression>,
    },
    Property {
        object: Box<Expression>,
        property: String,
    },
    ArrayLength(Box<Expression>),
    ArrayIncludes {
        array: Box<Expression>,
        value: Box<Expression>,
    },
    ArrayPush {
        array: Box<Expression>,
        value: Box<Expression>,
    },
    ArrayPop(Box<Expression>),
    ArrayGet {
        array: Box<Expression>,
        index: Box<Expression>,
    },
    ArrayJoin {
        array: Box<Expression>,
        separator: Box<Expression>,
    },
    OptionCheck {
        value: Box<Expression>,
        is_some: bool,
    },
    OptionUnwrapOr {
        value: Box<Expression>,
        fallback: Box<Expression>,
    },
    ResultCheck {
        value: Box<Expression>,
        is_ok: bool,
    },
    ResultUnwrapOr {
        value: Box<Expression>,
        fallback: Box<Expression>,
    },
    StringMethod {
        receiver: Box<Expression>,
        method: StringMethod,
        argument: Box<Expression>,
    },
    StringTransform {
        receiver: Box<Expression>,
        transform: StringTransform,
    },
    EnumVariant {
        enumeration: String,
        variant: String,
    },
    Binary {
        left: Box<Expression>,
        operator: BinaryOperator,
        right: Box<Expression>,
    },
    Unary {
        operator: UnaryOperator,
        value: Box<Expression>,
    },
    Conditional {
        condition: Box<Expression>,
        then_value: Box<Expression>,
        else_value: Box<Expression>,
    },
    Template {
        parts: Vec<String>,
        expressions: Vec<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringMethod {
    Includes,
    StartsWith,
    EndsWith,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StringTransform {
    Trim,
    ToUpperCase,
    ToLowerCase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
    Remainder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Not,
    Negate,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    Variable {
        name: String,
        mutable: bool,
        ty: Type,
        value: Expression,
    },
    Assignment {
        name: String,
        value: Expression,
    },
    Return(Expression),
    ReturnVoid,
    Break,
    Continue,
    Expression(Expression),
    ConsoleLog(Vec<Expression>),
    If {
        condition: Expression,
        then_body: Vec<Statement>,
        else_body: Vec<Statement>,
    },
    While {
        condition: Expression,
        body: Vec<Statement>,
    },
    ForOf {
        binding: String,
        iterable: Expression,
        body: Vec<Statement>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub is_async: bool,
    pub params: Vec<Parameter>,
    pub return_type: Type,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Program {
    pub structs: Vec<Struct>,
    pub enums: Vec<Enum>,
    pub functions: Vec<Function>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleImport {
    pub module: String,
    pub types: Vec<ImportBinding>,
    pub values: Vec<ImportBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImportBinding {
    pub imported: String,
    pub local: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Module {
    pub name: String,
    pub imports: Vec<ModuleImport>,
    pub reexports: Vec<ModuleImport>,
    pub exports: Vec<String>,
    pub default_export: Option<String>,
    pub program: Program,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    pub entry: String,
    pub modules: Vec<Module>,
}
