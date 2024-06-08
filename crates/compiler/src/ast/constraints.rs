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
    Number(f64),
    String(String),
    Boolean(bool),
    Map(Vec<(Constraint, Constraint)>),
    Ref(String),
}
