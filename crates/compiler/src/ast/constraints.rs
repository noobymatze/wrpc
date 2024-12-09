use std::collections::HashSet;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Constraint {
    Or(Vec<Constraint>),
    Lt(Vec<Constraint>),
    Eq(Vec<Constraint>),
    Le(Vec<Constraint>),
    Gt(Vec<Constraint>),
    Ge(Vec<Constraint>),
    And(Vec<Constraint>),
    Xor(Vec<Constraint>),
    Len(Box<Constraint>),
    Blank(Box<Constraint>),
    Not(Box<Constraint>),
    Number(f64),
    String(String),
    Boolean(bool),
    Map(Vec<(Constraint, Constraint)>),
    Access(String),
}

impl Constraint {
    /// Collect all properties this constraint references.
    ///
    /// This might include the property, that this constraint is
    /// attached to, itself. It needs to be excluded after calling this function.
    pub fn collect_accessed_deps(&self, deps: &mut HashSet<String>) {
        match self {
            Constraint::Access(name) => {
                deps.insert(name.clone());
            }
            Constraint::Or(constraints)
            | Constraint::Lt(constraints)
            | Constraint::Eq(constraints)
            | Constraint::Le(constraints)
            | Constraint::Gt(constraints)
            | Constraint::Ge(constraints)
            | Constraint::And(constraints)
            | Constraint::Xor(constraints) => {
                for constraint in constraints {
                    constraint.collect_accessed_deps(deps);
                }
            }
            Constraint::Len(boxed) | Constraint::Blank(boxed) | Constraint::Not(boxed) => {
                boxed.collect_accessed_deps(deps);
            }
            Constraint::Map(pairs) => {
                for (key, value) in pairs {
                    key.collect_accessed_deps(deps);
                    value.collect_accessed_deps(deps);
                }
            }
            Constraint::Number(_) | Constraint::String(_) | Constraint::Boolean(_) => {}
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Expr {
    And(Vec<Expr>),
    Or(Vec<Expr>),
    Xor(Vec<Expr>),
    Eq(Vec<Expr>),
    Lt(Vec<Expr>),
    Le(Vec<Expr>),
    Gt(Vec<Expr>),
    Ge(Vec<Expr>),
    Len(Box<Expr>),
    Not(Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Let(Box<Expr>, Box<Expr>),
    Number(f64),
    String(String),
    Boolean(bool),
    Map(Vec<(Expr, Expr)>),
    Get(Box<Expr>, String),
    Symbol(String),
}

#[cfg(test)]
mod tests {
    use crate::ast::constraints::Expr;

    #[test]
    fn test() {}
}

// Impl
// def blank = (lambda (expr) (= 0 (len expr)))
// (let ((a b)) )
