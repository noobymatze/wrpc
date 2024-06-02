use std::path::PathBuf;

use crate::ast::Module;
use crate::parse::lexer::{lexer, LexResult};

pub mod ast;
pub mod error;
mod parse;
pub mod reporting;

///
pub fn check(str: &str) -> Result<Module, error::Error> {
    let tokens = lexer(str).collect::<Vec<LexResult>>();
    println!("{tokens:?}");
    Ok(Module {
        doc_comment: None,
        version: "".to_string(),
        declarations: vec![],
    })
}

pub fn parse(filename: Option<PathBuf>, str: &str) -> Result<Module, error::Error> {
    parse::parse(filename, str).map_err(error::Error::BadSyntax)
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
