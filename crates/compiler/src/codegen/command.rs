pub use crate::codegen::kotlin::Options as KotlinOptions;
pub use crate::codegen::typescript::Options as TypescriptOptions;

#[derive(Debug)]
pub enum Command {
    Typescript(TypescriptOptions),
    Rust,
    Kotlin(KotlinOptions),
}
