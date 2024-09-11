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
    Ref(String),
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

fn kt(expr: &Expr, indent: &str) -> Result<String, ()> {
    match expr {
        Expr::And(expressions) => {
            if expressions.is_empty() {
                return Err(());
            } else {
                let mut result = vec![];
                for expr in expressions {
                    result.push(kt(expr, "")?);
                }

                Ok(result.join(" && "))
            }
        }
        Expr::Or(_) => todo!(),
        Expr::Xor(_) => todo!(),
        Expr::Eq(_) => todo!(),
        Expr::Lt(_) => todo!(),
        Expr::Le(_) => todo!(),
        Expr::Gt(_) => todo!(),
        Expr::Ge(_) => todo!(),
        Expr::Len(_) => todo!(),
        Expr::Not(_) => todo!(),
        Expr::If(_, _, _) => todo!(),
        Expr::Let(_, _) => todo!(),
        Expr::Number(_) => todo!(),
        Expr::String(_) => todo!(),
        Expr::Boolean(bool) => Ok(format!("{}", bool)),
        Expr::Map(_) => todo!(),
        Expr::Get(_, _) => todo!(),
        Expr::Symbol(value) => Ok(format!("{value}")),
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::constraints::Expr;

    use super::kt;

    #[test]
    fn test() {
        let expr = Expr::And(vec![Expr::Boolean(true), Expr::Boolean(true)]);
        println!("{:?}", kt(&expr, ""));
    }
}

// Impl
// def blank = (lambda (expr) (= 0 (len expr)))
// (let ((a b)) )
