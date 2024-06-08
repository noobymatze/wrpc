use crate::ast::source::Module;

mod typescript;

pub fn generate_typescript_client(_module: &Module) -> Result<(), ()> {
    //for decl in module.declarations.iter() {}

    Ok(())
}
