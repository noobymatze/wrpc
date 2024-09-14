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
