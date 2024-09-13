use itertools::Itertools;

use crate::ast::{
    canonical::{Module, Record, Type},
    constraints::Constraint,
};

#[derive(Debug, Clone)]
enum Lit {
    String(String),
    Int32(i32),
    Int64(i64),
    Float64(f64),
    Bool(bool),
}

#[derive(Debug, Clone)]
enum Json {
    Object,
    Int,
    Array,
    Float,
    String,
    Null,
}

#[derive(Debug, Clone)]
enum JsonOp {
    Is(Json),
    Get(Json),
}

#[derive(Debug, Clone)]
enum InfixOp {
    Plus,
    Minus,
    Mult,
    Div,
    Or,
    And,
    Equals,
    NotEquals,
    GreaterThan,
    LesserThan,
    GreaterEquals,
    LesserEquals,
}

#[derive(Debug, Clone)]
enum PrefixOp {
    Not,
    Minus,
}

#[derive(Debug, Clone)]
enum Expr {
    Var(String),
    Lit(Lit),
    Prefix(PrefixOp, Box<Expr>),
    Infix(InfixOp, Box<Expr>, Box<Expr>),
    Json(JsonOp, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Access(Box<Expr>, String),
}

#[derive(Debug, Clone)]
enum Stmt {
    Expr(Expr),
    Assign(String, Expr),
    Return(Expr),
    Block(Vec<Stmt>),
    Data(Record),
    Enum(String, Vec<String>),
    Function(String, Vec<(String, Type)>, Option<Type>, Vec<Stmt>),
}

#[derive(Debug, Clone)]
struct File {
    statments: Vec<Stmt>,
}

pub fn from_canonical(module: &Module) -> Result<File, ()> {
    let mut stmts = vec![];
    for (_, record) in &module.records {
        stmts.push(Stmt::Data(record.clone()));
        stmts.push(validate_record(record));
    }

    Ok(File { statments: stmts })
}

pub fn validate_record(record: &Record) -> Stmt {
    let name = format!("validate{}", record.name.value);
    let mut stmts = vec![];
    for annotation in &record.annotations {
        for constraint in annotation.get_constraints() {
            stmts.push(Stmt::Expr(constraint_to_expr(&name, &constraint)));
        }
    }

    Stmt::Function(
        name,
        vec![(
            "value".to_string(),
            Type::Ref(record.name.value.clone(), vec![]),
        )],
        None,
        stmts,
    )
}

fn combine(op: InfixOp, expressions: &Vec<Expr>) -> Option<Expr> {
    expressions.iter().fold(None, |result, next| match result {
        Some(left) => Some(Expr::Infix(op.clone(), left.into(), next.clone().into())),
        None => Some(next.clone()),
    })
}

pub fn constraint_to_expr(name: &String, constraint: &Constraint) -> Expr {
    match constraint {
        Constraint::Ref(property) => Expr::Access(Expr::Var(name.clone()).into(), property.clone()),
        Constraint::Or(constraints) => {
            let expressions = constraints
                .iter()
                .map(|constraint| constraint_to_expr(name, constraint))
                .collect::<Vec<Expr>>();
            combine(InfixOp::Or, &expressions).unwrap()
        }
        Constraint::And(constraints) => {
            let expressions = constraints
                .iter()
                .map(|constraint| constraint_to_expr(name, constraint))
                .collect::<Vec<Expr>>();
            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Lt(constraints) => {
            let expressions = constraints
                .iter()
                .zip(constraints.iter().skip(1))
                .map(|(a, b)| {
                    Expr::Infix(
                        InfixOp::LesserThan,
                        constraint_to_expr(name, a).into(),
                        constraint_to_expr(name, b).into(),
                    )
                })
                .collect::<Vec<Expr>>();

            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Eq(constraints) => {
            let expressions = constraints
                .iter()
                .zip(constraints.iter().skip(1))
                .map(|(a, b)| {
                    Expr::Infix(
                        InfixOp::Equals,
                        constraint_to_expr(name, a).into(),
                        constraint_to_expr(name, b).into(),
                    )
                })
                .collect::<Vec<Expr>>();

            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Le(constraints) => {
            let expressions = constraints
                .iter()
                .zip(constraints.iter().skip(1))
                .map(|(a, b)| {
                    Expr::Infix(
                        InfixOp::LesserEquals,
                        constraint_to_expr(name, a).into(),
                        constraint_to_expr(name, b).into(),
                    )
                })
                .collect::<Vec<Expr>>();

            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Gt(constraints) => {
            let expressions = constraints
                .iter()
                .zip(constraints.iter().skip(1))
                .map(|(a, b)| {
                    Expr::Infix(
                        InfixOp::GreaterThan,
                        constraint_to_expr(name, a).into(),
                        constraint_to_expr(name, b).into(),
                    )
                })
                .collect::<Vec<Expr>>();

            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Ge(constraints) => {
            let expressions = constraints
                .iter()
                .zip(constraints.iter().skip(1))
                .map(|(a, b)| {
                    Expr::Infix(
                        InfixOp::GreaterEquals,
                        constraint_to_expr(name, a).into(),
                        constraint_to_expr(name, b).into(),
                    )
                })
                .collect::<Vec<Expr>>();

            combine(InfixOp::And, &expressions).unwrap()
        }
        Constraint::Xor(_) => todo!(),
        Constraint::Len(_) => todo!(),
        Constraint::Blank(_) => todo!(),
        Constraint::Not(constraint) => {
            Expr::Prefix(PrefixOp::Not, constraint_to_expr(name, constraint).into())
        }
        Constraint::Number(value) => Expr::Lit(Lit::Float64(*value)),
        Constraint::String(value) => Expr::Lit(Lit::String(value.clone())),
        Constraint::Boolean(value) => Expr::Lit(Lit::Bool(*value)),
        Constraint::Map(_) => todo!(),
    }
}

fn render_kotlin_file(file: &File) -> String {
    let indent = "".to_string();
    let result = file
        .statments
        .iter()
        .map(|stmt| render_kotlin_stmt(&indent, stmt))
        .join("\n\n");

    result
}

fn render_kotlin_stmt(indent: &String, stmt: &Stmt) -> String {
    match stmt {
        Stmt::Expr(expr) => render_kotlin_expr(indent, expr),
        Stmt::Assign(name, expr) => format!("val {name} = {}", render_kotlin_expr(indent, expr)),
        Stmt::Return(expr) => format!("return {}", render_kotlin_expr(indent, expr)),
        Stmt::Block(stmts) => {
            let statments = stmts
                .iter()
                .map(|stmt| render_kotlin_stmt(indent, stmt))
                .join("\n");

            format!("{statments}")
        }
        Stmt::Data(record) => {
            let fields = record
                .properties
                .iter()
                .map(|property| {
                    format!(
                        "    val {}: {}",
                        property.name.value,
                        render_kotlin_type_ref(&"".to_string(), &property.type_)
                    )
                })
                .join(",\n");

            format!("data class {}(\n{fields}\n)", record.name.value)
        }
        Stmt::Enum(_, _) => "".to_string(),
        Stmt::Function(name, params, return_type, body) => {
            let params = params
                .iter()
                .map(|param| {
                    format!(
                        "{}: {}",
                        param.0,
                        render_kotlin_type_ref(&"".to_string(), &param.1)
                    )
                })
                .join(", ");

            let statements = body
                .iter()
                .map(|stmt| render_kotlin_stmt(&"    ".to_string(), stmt))
                .join("\n");

            let return_type = return_type
                .as_ref()
                .map(|type_| format!(": {}", render_kotlin_type_ref(&"".to_string(), type_)))
                .unwrap_or("".to_string());

            format!("fun {name}({params}){return_type} {{\n{statements}\n}}")
        }
    }
}

fn render_kotlin_expr(indent: &String, expr: &Expr) -> String {
    match expr {
        Expr::Var(name) => name.clone(),
        Expr::Lit(Lit::Int32(value)) => format!("{value}"),
        Expr::Lit(Lit::Int64(value)) => format!("{value}"),
        Expr::Lit(Lit::Float64(value)) => format!("{value}"),
        Expr::Lit(Lit::Bool(value)) => format!("{value}"),
        Expr::Lit(Lit::String(value)) => format!("{value}"),
        Expr::Json(op, expr) => match op {
            JsonOp::Is(Json::Object) => {
                let expr = render_kotlin_expr(&"".to_string(), expr);
                format!("{expr} is JsonObject")
            }
            JsonOp::Is(_) => {
                format!("{:?}", expr)
            }
            JsonOp::Get(_) => format!("{:?}", expr),
        },
        Expr::Prefix(op, expr) => {
            let expr = render_kotlin_expr(indent, expr);
            match op {
                PrefixOp::Not => format!("!{expr}"),
                PrefixOp::Minus => format!("-{expr}"),
            }
        }
        Expr::Infix(op, left, right) => {
            let left = render_kotlin_expr(indent, left);
            let right = render_kotlin_expr(indent, right);
            let op = match op {
                InfixOp::Plus => "+",
                InfixOp::Minus => "-",
                InfixOp::Mult => "*",
                InfixOp::Div => "/",
                InfixOp::And => "&&",
                InfixOp::Or => "||",
                InfixOp::Equals => "==",
                InfixOp::NotEquals => "!=",
                InfixOp::GreaterThan => ">",
                InfixOp::LesserThan => "<",
                InfixOp::GreaterEquals => ">=",
                InfixOp::LesserEquals => "<=",
            };

            format!("({left} {op} {right})")
        }
        Expr::If(condition, then_, else_) => format!(
            "if ({}) {} else {}",
            render_kotlin_expr(indent, condition),
            render_kotlin_expr(indent, then_),
            render_kotlin_expr(indent, else_)
        ),
        Expr::Call(expr, parameters) => {
            let fun = render_kotlin_expr(indent, expr);
            let params = parameters
                .iter()
                .map(|expr| render_kotlin_expr(indent, expr))
                .join(", ");

            format!("{fun}({params})")
        }
        Expr::Access(expr, name) => format!("{}.{}", render_kotlin_expr(indent, expr), name),
    }
}

fn render_kotlin_type_ref(package: &String, type_: &Type) -> String {
    match type_ {
        Type::String => "kotlin.String".to_string(),
        Type::Boolean => "kotlin.Boolean".to_string(),
        Type::Int32 => "kotlin.Int".to_string(),
        Type::Int64 => "kotlin.Long".to_string(),
        Type::Float32 => "kotlin.Float".to_string(),
        Type::Float64 => "kotlin.Double".to_string(),
        Type::Map(key_type, value_type) => {
            let key = render_kotlin_type_ref(package, key_type);
            let value = render_kotlin_type_ref(package, value_type);
            format!("kotlin.Map<{key}, {value}>")
        }
        Type::Result(error_type, value_type) => {
            let error = render_kotlin_type_ref(package, error_type);
            let value = render_kotlin_type_ref(package, value_type);
            format!("Result<{error}, {value}>")
        }
        Type::List(value_type) => {
            let value = render_kotlin_type_ref(package, value_type);
            format!("kotlin.collections.List<{value}>")
        }
        Type::Set(value_type) => {
            let value = render_kotlin_type_ref(package, value_type);
            format!("kotlin.collections.Set<{value}>")
        }
        Type::Option(value_type) => {
            let value = render_kotlin_type_ref(package, value_type);
            format!("{value}?")
        }
        Type::Ref(name, _) => name.clone(),
    }
}

fn render_kotlin_doc_comment(indent: &str, comment: &Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(comment) => {
            let content = comment
                .split("\n")
                .map(|line| format!("{indent} * {line}"))
                .collect::<Vec<String>>()
                .join("\n");
            format!("{indent}/**\n{content}\n{indent} */\n")
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ast::{
            canonical::{Record, Type},
            constraints::Constraint,
            source::Name,
        },
        codegen::ast::{
            constraint_to_expr, render_kotlin_expr, render_kotlin_file, render_kotlin_stmt, Expr,
            File, Json, JsonOp, Lit, Stmt,
        },
        reporting::Region,
    };

    #[test]
    fn test() {
        let constraint = Constraint::And(vec![
            Constraint::Lt(vec![
                Constraint::Number(5.0),
                Constraint::Number(6.0),
                Constraint::Ref("test".to_string()),
            ]),
            Constraint::Eq(vec![Constraint::Number(5.0), Constraint::Number(6.0)]),
        ]);

        let expr = Expr::If(
            Box::new(Expr::Json(
                JsonOp::Is(Json::Object),
                Box::new(Expr::Var("value".to_string())),
            )),
            Expr::Var("test".to_string()).into(),
            Expr::Var("test2".to_string()).into(),
        );

        let value = "value".to_string();
        let result = render_kotlin_expr(&"".to_string(), &expr);
        println!("{}", result);

        let name = Name {
            region: Region::line(1, 0, 5),
            value: "Hello".to_string(),
        };

        let record = Record {
            annotations: vec![],
            comment: None,
            name: name,
            properties: vec![],
            type_variables: vec![],
        };
    }
}
