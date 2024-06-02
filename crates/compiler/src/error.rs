/// ! Hello World
use serde::{Deserialize, Serialize};

pub mod syntax;

///
#[derive(Debug, Serialize, Deserialize)]
pub enum Error {
    BadSyntax(Vec<syntax::Error>),
}
