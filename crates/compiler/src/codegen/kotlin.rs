use crate::ast::canonical::Parameter;
use crate::ast::constraints::Constraint;
use crate::ast::{
    canonical::{Enum, Method, Module, Property, Record, Service, Type, Variant},
    source::Name,
};
use askama::Template; // bring trait in scope
use itertools::Itertools;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::string::ToString;
use std::{fs, io};

const OPEN: &'static str = "{";
const CLOSE: &'static str = "}";

#[derive(Debug)]
pub struct Options {
    pub print: bool,
    pub output: Option<PathBuf>,
    pub package: String,
}

#[derive(Debug)]
pub struct KtFile {
    folder: String,
    name: String,
    content: String,
}

pub fn generate_kotlin_server(module: &Module, options: &Options) -> Result<(), io::Error> {
    let record_package = &options.package;
    //for decl in module.declarations.iter() {}
    let mut files = vec![];
    for (_, record) in &module.records {
        files.push(KtFile {
            name: format!("{}.kt", record.name.value),
            folder: "models".to_string(),
            content: generate_record(&record_package, record, true),
        });
    }

    for (_, enum_value) in &module.enums {
        files.push(KtFile {
            name: format!("{}.kt", enum_value.name.value),
            folder: "models".to_string(),
            content: generate_enum(&record_package, enum_value),
        });
    }

    for (_, service) in &module.services {
        files.push(KtFile {
            name: format!("{}.kt", service.name.value),
            folder: "services".to_string(),
            content: generate_service(&record_package, service),
        });
    }

    files.push(KtFile {
        name: "json.kt".to_string(),
        folder: "json".to_string(),
        content: generate_json_functions(record_package),
    });

    files.push(KtFile {
        name: "result.kt".to_string(),
        folder: "models".to_string(),
        content: generate_result_type(record_package),
    });

    if options.print {
        for file in &files {
            println!("{}", file.content);
        }
    }

    if let Some(out) = &options.output {
        fs::create_dir_all(out)?;
        fs::create_dir_all(out.join("json"))?;
        fs::create_dir_all(out.join("models"))?;
        fs::create_dir_all(out.join("services"))?;

        for kt_file in files {
            let mut file = File::create(out.join(kt_file.folder).join(kt_file.name))?;
            file.write_all(kt_file.content.as_bytes())?;
        }
    }

    Ok(())
}

fn generate_service(package: &String, service: &Service) -> String {
    ServiceTemplate { service, package }
        .render()
        .expect("Should work.")
}

fn generate_record(package: &String, record: &Record, with_imports: bool) -> String {
    RecordTemplate { record, package }
        .render()
        .expect("Should work.")
}

fn generate_enum(package: &String, record: &Enum) -> String {
    EnumTemplate { record, package }
        .render()
        .expect("Should work.")
}

#[derive(Template)]
#[template(path = "kotlin/record.kt", escape = "txt")]
struct RecordTemplate<'a> {
    record: &'a Record,
    package: &'a String,
}

#[derive(Template)]
#[template(path = "kotlin/enum.kt", escape = "txt")]
struct EnumTemplate<'a> {
    record: &'a Enum,
    package: &'a String,
}

#[derive(Template)]
#[template(path = "kotlin/service.kt", escape = "txt")]
struct ServiceTemplate<'a> {
    service: &'a Service,
    package: &'a String,
}

pub fn generate_type_ref(package: &String, type_: &Type) -> String {
    match type_ {
        Type::String => "kotlin.String".to_string(),
        Type::Boolean => "kotlin.Boolean".to_string(),
        Type::Int32 => "kotlin.Int".to_string(),
        Type::Int64 => "kotlin.Long".to_string(),
        Type::Float32 => "kotlin.Float".to_string(),
        Type::Float64 => "kotlin.Double".to_string(),
        Type::Map(key_type, value_type) => {
            let key = generate_type_ref(package, key_type);
            let value = generate_type_ref(package, value_type);
            format!("kotlin.Map<{key}, {value}>")
        }
        Type::Result(error_type, value_type) => {
            let error = generate_type_ref(package, error_type);
            let value = generate_type_ref(package, value_type);
            format!("Result<{error}, {value}>")
        }
        Type::List(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("kotlin.collections.List<{value}>")
        }
        Type::Set(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("kotlin.collections.Set<{value}>")
        }
        Type::Option(value_type) => {
            let value = generate_type_ref(package, value_type);
            format!("{value}?")
        }
        Type::Ref(name, types) => {
            if types.is_empty() {
                name.clone()
            } else {
                let refs = types
                    .iter()
                    .map(|type_| generate_type_ref(package, type_))
                    .join(", ");

                format!("{name}<{refs}>")
            }
        }
    }
}

fn generate_doc_comment(indent: &str, comment: &Option<String>) -> String {
    match comment {
        None => "".to_string(),
        Some(comment) => {
            let content = comment
                .split("\n")
                .map(|line| format!("{indent} * {line}"))
                .collect::<Vec<String>>()
                .join("\n");
            format!("{indent}/**\n{content}\n{indent} */")
        }
    }
}

fn encode_type(var_expr: &str, type_: &Type) -> String {
    match type_ {
        Type::String => var_expr.to_string(),
        Type::Boolean => var_expr.to_string(),
        Type::Int32 => var_expr.to_string(),
        Type::Int64 => var_expr.to_string(),
        Type::Float32 => var_expr.to_string(),
        Type::Float64 => var_expr.to_string(),
        Type::Map(_, _) => var_expr.to_string(),
        Type::Result(error_type, ok_type) => format!(
            "{var_expr}.encode(encodeOk = {{ {} }}, encodeErr = {{ {} }})",
            encode_type(&"it".to_string(), ok_type),
            encode_type(&"it".to_string(), error_type)
        ),
        Type::List(type_) => format!(
            "buildJsonArray {{ {var_expr}.forEach {{ add({}) }} }}",
            encode_type(&"it".to_string(), type_)
        ),
        Type::Set(type_) => format!(
            "buildJsonArray {{ {var_expr}.forEach {{ add({}) }} }}",
            encode_type(&"it".to_string(), type_)
        ),
        Type::Option(type_) => encode_type(&format!("{var_expr}"), type_),
        Type::Ref(name, _) => format!("{var_expr}.encode()"),
    }
}

// JSON DECODE

fn decode_property(indent: &str, var_object: &str, var_error: &str, property: &Property) -> String {
    let var_name = &property.name.value;
    let field_name = &property.name.value;
    let var_field = format!("{var_name}Field");

    [
        format!("{indent}val {var_field} = {var_object}[\"{field_name}\"]"),
        decode_type(
            indent,
            &var_field,
            var_name,
            &var_error.to_string(),
            &property.type_,
            true,
            |err| format!("errors.field(\"{field_name}\", {err})"),
        ),
    ]
    .join("\n")
}

fn decode_parameter(
    indent: &str,
    var_object: &str,
    var_error: &str,
    property: &Parameter,
) -> String {
    let var_name = &property.name.value;
    let field_name = &property.name.value;
    let var_field = format!("{var_name}Field");

    [
        format!("{indent}val {var_field} = {var_object}[\"{field_name}\"]"),
        decode_type(
            indent,
            &var_field,
            var_name,
            &var_error.to_string(),
            &property.type_,
            true,
            |err| format!("errors.field(\"{field_name}\", {err})"),
        ),
    ]
    .join("\n")
}

fn decode_type<F>(
    indent: &str,
    var_json: &String,
    var_name: &String,
    var_error: &String,
    type_: &Type,
    required: bool,
    error: F,
) -> String
where
    F: Fn(String) -> String,
{
    let type_name = match &type_ {
        Type::Option(type_) => generate_type_ref(&"".to_string(), &type_),
        _ => generate_type_ref(&"".to_string(), &type_),
    };

    let not_null_error = if required {
        [
            format!("else {OPEN}"),
            format!(
                "{indent}    {var_error}.error({})",
                error(format!("{var_error}.notNull"))
            ),
            format!("{indent}{CLOSE}"),
        ]
        .join("\n")
    } else {
        "".to_string()
    };

    match &type_ {
        Type::String => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive && {var_json}.isString) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.content"),
            format!("{indent}    {CLOSE} else {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"STRING\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Int32 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.intOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"INT32\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Int64 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.longOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"INT64\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Float32 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.floatOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"FLOAT32\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Float64 => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.doubleOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"FLOAT64\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Boolean => [
            format!("{indent}var {var_name}: {type_name}? = null"),
            format!("{indent}if ({var_json} != null) {OPEN}"),
            format!("{indent}    if ({var_json} is JsonPrimitive) {OPEN}"),
            format!("{indent}        {var_name} = {var_json}.booleanOrNull"),
            format!("{indent}    {CLOSE}"),
            format!("{indent}    if ({var_name} == null) {OPEN}"),
            format!(
                "{indent}        {var_error}.error({})",
                error(format!("{var_error}.expect(\"BOOLEAN\")"))
            ),
            format!("{indent}    {CLOSE}"),
            format!("{indent}{CLOSE} {not_null_error}"),
        ]
        .join("\n"),
        Type::Option(type_) => {
            decode_type(indent, var_json, var_name, var_error, type_, false, error)
        }
        Type::Ref(name, _) => [
            format!("{indent}val {var_name}Errors = Errors()"),
            format!("{indent}val {var_name} = {name}.decode({var_json}, {var_name}Errors)"),
            format!("{indent}{var_error}.error({var_name}Errors)"),
        ]
        .join("\n"),
        _ => format!("not implemented: {type_:?}"),
    }
}

fn generate_json_functions(package: &String) -> String {
    let data = include_str!("kotlin/json.kt");
    format!("package {package}.json\n\n{data}")
}

fn generate_result_type(package: &String) -> String {
    let data = include_str!("kotlin/result.kt");
    let imports = r#"
import {package}.json.*
import kotlinx.serialization.json.*
    "#;
    format!("package {package}.models{imports}\n\n{data}")
}

fn condition(var_expr: &str, constraint: &Constraint) -> String {
    match constraint {
        Constraint::Or(constraints) => {
            if constraints.is_empty() {
                condition(var_expr, &Constraint::Boolean(true))
            } else {
                constraints
                    .iter()
                    .map(|constraint| condition(var_expr, constraint))
                    .join(" || ")
            }
        }
        Constraint::And(constraints) => {
            if constraints.is_empty() {
                condition(var_expr, &Constraint::Boolean(false))
            } else {
                constraints
                    .iter()
                    .map(|constraint| condition(var_expr, constraint))
                    .join(" && ")
            }
        }
        Constraint::Eq(constraints) => binop("==", var_expr, constraints),
        Constraint::Lt(constraints) => binop("<", var_expr, constraints),
        Constraint::Le(constraints) => binop("<=", var_expr, constraints),
        Constraint::Gt(constraints) => binop(">", var_expr, constraints),
        Constraint::Ge(constraints) => binop(">=", var_expr, constraints),
        Constraint::Xor(_) => todo!(),
        Constraint::Len(constraint) => {
            format!("{}.size", condition(var_expr, constraint))
        }
        Constraint::Blank(constraint) => {
            let constraint = Constraint::Eq(vec![
                Constraint::Len(constraint.clone()),
                Constraint::Number(0.0),
            ]);
            condition(var_expr, &constraint)
        }
        Constraint::Not(constraint) => format!("!{}", condition(var_expr, constraint)),
        Constraint::Number(value) => format!("{value}"),
        Constraint::String(value) => format!("\"{value}\""),
        Constraint::Boolean(value) => format!("{value}"),
        Constraint::Map(entries) => format!(
            "mapOf({})",
            entries
                .iter()
                .map(|(key, value)| format!(
                    "{} to {}",
                    condition(var_expr, key),
                    condition(var_expr, value)
                ))
                .join(", ")
        ),
        Constraint::Access(value) => {
            format!("{var_expr}.{value}")
        }
    }
}

fn binop(op: &str, var_expr: &str, constraints: &Vec<Constraint>) -> String {
    if constraints.is_empty() {
        condition(var_expr, &Constraint::Boolean(false))
    } else if constraints.len() == 1 {
        constraints
            .iter()
            .map(|constraint| condition(var_expr, constraint))
            .join("")
    } else {
        constraints
            .iter()
            .zip(constraints.iter().skip(1))
            .map(|(a, b)| {
                let a = condition(var_expr, a);
                let b = condition(var_expr, b);
                format!("({a} {op} {b})")
            })
            .join(" && ")
    }
}

#[cfg(test)]
mod tests {
    use crate::ast::constraints::Constraint;
    use crate::codegen::kotlin::{generate_record, EnumTemplate, RecordTemplate, ServiceTemplate};
    use crate::compile;
    use crate::error::Error;
    use askama::Template; // bring trait in scope

    #[test]
    fn test() -> Result<(), Error> {
        let spec = r#"
            data Address {
                #(check (not (blank .name)))
                street: String,
                houseNo: Int32,
                #(check
                    (or (and (= .country "DE") (= (len .zipcode) 5))
                        (and (= .country "CH") (= (len .zipcode) 4))))
                zipcode: String,
                #(check (or (= .country "DE") (= .country "CH")))
                country: String,
            }

            enum LoginResult {
                Hello { name: String },
                Test { name: String },
            }

            enum Country {
                DE,
                EN,
                CH,
            }

            // The [SessionService] manages sessions and allows a
            // user to login.
            service SessionService {

                // Signs in a user based on the given [Credentials](#Credentials).
                def login(credentials: Credentials): Greet

                // Sign out the given session via the given SessionId.
                def logout(session: SessionId): Result<Error, Greet>

            }
        "#;

        let module = compile(None, spec)?;
        let result = module.records.get("Address").expect("Get Address");
        let login_result = module.enums.get("LoginResult").expect("Get LoginResult");
        let country = module.enums.get("Country").expect("Get Country");
        let package = "test".to_string();

        let enum_template = EnumTemplate {
            record: login_result,
            package: &package,
        };
        println!("{}", enum_template.render().unwrap());

        let service_template = ServiceTemplate {
            service: module
                .services
                .get("SessionService")
                .expect("Get SessionService"),
            package: &package,
        };
        println!("{}", service_template.render().unwrap());

        let len = Constraint::Len(Box::new(Constraint::Access("test".to_string())));
        let leq = Constraint::Le(vec![
            len.clone(),
            Constraint::Number(4.0),
            Constraint::Number(5.0),
        ]);

        //println!("{}", validate_value("", "test", "errors", &vec![leq]));
        println!("{}", generate_record(&package, &result, true));

        Ok(())
    }
}
