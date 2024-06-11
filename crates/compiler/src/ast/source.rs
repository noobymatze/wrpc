use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display, Formatter};

use crate::reporting::Region;

/// A `Module` contains a number of
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Module {
    pub doc_comment: Option<String>,
    pub version: String,
    pub declarations: Vec<Decl>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Decl {
    Data(Data),
    Enum(Enum),
    Service(Service),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Data {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub properties: Vec<Property>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Property {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub type_: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Enum {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub variants: Vec<Variant>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Variant {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub properties: Vec<Property>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Type {
    pub name: Name,
    pub variables: Vec<Type>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TypeVariable {
    pub name: Name,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Service {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub methods: Vec<Method>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Method {
    pub annotations: Vec<Annotation>,
    pub doc_comment: Option<String>,
    pub name: Name,
    pub parameters: Vec<Parameter>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Parameter {
    pub annotations: Vec<Annotation>,
    pub name: Name,
    pub type_: Type,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Name {
    #[serde(skip_serializing)]
    pub region: Region,
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Annotation {
    pub expr: Expr,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Expr {
    Boolean(Region, bool),
    Number(Region, f64),
    String(Region, String),
    Keyword(Region, String),
    Symbol(Region, String),
    List(Region, Vec<Expr>),
    Map(Region, Vec<(Expr, Expr)>),
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Expr::Boolean(_, value) => write!(f, "{}", value),
            Expr::Number(_, value) => write!(f, "{}", value),
            Expr::String(_, value) => write!(f, "\"{}\"", value),
            Expr::Keyword(_, value) => write!(f, ":{}", value),
            Expr::Symbol(_, value) => write!(f, "{}", value),
            Expr::Map(_, value) => write!(f, "{:?}", value),
            Expr::List(_, expressions) => write!(
                f,
                "({})",
                expressions
                    .iter()
                    .map(|expr| format!("{}", expr))
                    .collect::<Vec<String>>()
                    .join(" ")
            ),
        }
    }
}

impl Name {
    pub fn uncapitalized(&self) -> String {
        let mut c = self.value.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_lowercase().collect::<String>() + c.as_str(),
        }
    }

    pub fn capitalized(&self) -> String {
        let mut c = self.value.chars();
        match c.next() {
            None => String::new(),
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        }
    }

    pub fn request_name(&self) -> String {
        let value = self.capitalized();
        format!("{value}Request")
    }
}
