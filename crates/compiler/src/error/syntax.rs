use serde::{Deserialize, Serialize};

use crate::ast;
use crate::reporting::{Col, Line, Position, Report, WrpcDocBuilder};
/// ! This module contains all potential syntax errors
/// ! of the wRPC language.
use crate::{parse, reporting::Region};

/// A convenience [`Result`][Result] for working with
/// syntax errors.
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Error {
    ParseError(Module),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Module {
    Decl(Decl),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Decl {
    DataName(Name),
    Start(Line, Col),
    MissingPropertySeparator(Region),
    BadComment(Token),
    BadProperty(Property),
    End(usize, usize),
    BadService(Service),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Service {
    BadName(Name),
    BadMethod(Method),
    Start(Line, Col),
    MissingEnd(usize, usize),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Method {
    BadName(Name),
    BadComment(Token),
    BadParam(Property),
    BadReturnType(Type),
    MissingDef(Line, Col),
    MissingParamStart(ast::Name, Line, Col),
    MissingParamEnd(ast::Name, Line, Col),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Type {
    BadName(Name),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Property {
    BadName(Name),
    BadType(ast::Name, Type),
    BadComment(Token),
    MissingComma(Region),
    MissingType(Region),
    MissingColon(ast::Name, Line, Col),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Token {
    String(Line, Col, Str),
    Number(Line, Col, Number),
    Comment(Comment, Line, Col),
    BadChar(Line, Col, char),
    Eof(Line, Col),
}

impl Token {
    pub fn position(&self) -> Position {
        let (line, col) = match self {
            Token::String(line, col, _) => (line, col),
            Token::Number(line, col, _) => (line, col),
            Token::Comment(_, line, col) => (line, col),
            Token::BadChar(line, col, _) => (line, col),
            Token::Eof(line, col) => (line, col),
        };

        Position {
            line: *line,
            col: *col,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Name {
    BadToken(Token),
    ExpectedName(Line, Col),
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub enum Expr {
    String(Str, Line, Col),
    Number(Number, Line, Col),
    BadToken(Region, parse::token::Token),
    Endless(Line, Col),
    BadChar(char, Line, Col),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Str {
    Endless,
    StringEscape(Escape),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Comment {
    Start,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Number {
    Bad(String),
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
pub enum Escape {
    EscapeUnknown,
}

impl Error {
    pub fn to_report<'a>(&self, alloc: &'a WrpcDocBuilder) -> Report<'a> {
        match self {
            Error::ParseError(module) => module.to_report(alloc),
        }
    }
}

impl Module {
    pub fn to_report<'a>(&self, alloc: &'a WrpcDocBuilder) -> Report<'a> {
        match self {
            Module::Decl(decl) => decl.to_report(alloc),
        }
    }
}

impl Decl {
    pub fn to_report<'a>(&self, alloc: &'a WrpcDocBuilder) -> Report<'a> {
        match self {
            Decl::BadComment(_) => Report { title: "COMMENT SYNTAX".to_owned(), doc: alloc.stack([alloc.reflow("Test"), alloc.reflow("Mehr Test")]) },
            Decl::End(line, col) => Report {
                title: "UNEXPECTED END OF DATA DECLARATION".to_owned(),
                 doc: alloc.stack([
                    alloc.reflow("I tried to parse a `data` declaration but missed an ending curly brace."),
                    alloc.snippet(&Region::from_position(&Position { line: *line, col: *col }, &Position { line: *line, col: *col }))
                ])
            },
            Decl::Start(line, col) => Report {
                title: "DATA DECLARATION".to_owned(),
                 doc: alloc.stack([
                     alloc.reflow("I tried to read a data declaration"),
                     alloc.snippet(&Region::new(*line, *col, *line, *col))
                 ])
                },
            Decl::DataName(Name::BadToken(_)) => Report {
                title: "DATA DECLARATION".to_owned(),
                 doc: alloc.stack([alloc.reflow("Test")])
                },

            Decl::DataName(Name::ExpectedName(_, _)) => Report {
                title: "DATA DECLARATION".to_owned(),
                 doc: alloc.stack([alloc.reflow("Test")])
                },
            Decl::BadProperty(Property::MissingComma(_)) => Report {
                title: "MISSING PROPERTY NAME".to_string(),
                //region: region.clone(),
                doc: alloc.stack([
                    alloc.reflow("I am missing a comma in a ")
                ]),
            },
            Decl::MissingPropertySeparator(region) => Report {
                title: "MISSING PROPERTY SEPARATOR".to_string(),
                doc: alloc.stack([
                    alloc.reflow("I missed a separator between two properties."),
                    alloc.snippet(region),
                    alloc.reflow("Properties can be declared in the form of `name: Type,`. Please add a comma.")
                ])
            },
            Decl::BadProperty(Property::MissingColon(_name, line, col)) => Report {
                title: "MISSING PROPERTY NAME AND TYPE SEPARATOR".to_string(),
                doc: alloc.stack([
                    alloc.reflow(format!("I found a property with the name `{}`", "Test")),
                    alloc.snippet(&Region::line(*line, *col, *col)),
                ])
            },
            Decl::BadProperty(Property::MissingType(region)) => Report {
                title: "MISSING PROPERTY TYPE".to_string(),
                doc: alloc.stack([
                    alloc.reflow(format!(
                        "I found a property with the name `{}`, but \
                             cannot find a type associated with this property.",
                        "Test",
                    )),
                    alloc.snippet(region),
                ])
            },
            Decl::BadService(_) => Report { title: "BAD SERVICE DECLARATION".to_owned(), doc: alloc.stack([alloc.reflow("TEST SERVICE")]) },
            Decl::BadProperty(Property::BadName(_)) => Report { title: "BAD PROPERTY NAME".to_owned(), doc: alloc.stack([alloc.reflow("TEST")]) },
            Decl::BadProperty(Property::BadType(_, _)) => Report { title: "BAD PROPERTY TYPE".to_owned(), doc: alloc.stack([alloc.reflow("TEST")]) },
            Decl::BadProperty(Property::BadComment(_)) => Report { title: "BAD PROPERTY COMMENT".to_owned(), doc: alloc.stack([alloc.reflow("TEST")]) },
        }
    }
}
