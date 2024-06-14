pub use crate::codegen::typescript::Options as TypescriptOptions;

#[derive(Debug)]
pub enum Command {
    Typescript(TypescriptOptions),
    Rust,
    Kotlin,
}
