use crate::ast::canonical::{Enum, Module, Parameter, Record, Service, Type};

use askama::Template;
use itertools::Itertools; // bring trait in scope

#[derive(Template)] // this will generate the code...
#[template(path = "index.html")] // using the template in this path, relative
                                 // to the `templates` dir in the crate root
struct DocTemplate<'a> {
    module: &'a Module,
}

pub fn md_to_html(val: &String) -> String {
    markdown::to_html(val)
}

pub fn render_return_type(type_: &Option<Type>) -> String {
    match type_ {
        Some(type_) => format!(": {}", render_type(type_)),
        None => "".to_string(),
    }
}

pub fn render_parameters(parameters: &Vec<Parameter>) -> String {
    parameters
        .iter()
        .map(|parameter| {
            format!(
                "{}: {}",
                parameter.name.value,
                render_type(&parameter.type_)
            )
        })
        .join(", ")
}

pub fn render_type(type_: &Type) -> String {
    match type_ {
        Type::String => format!("<span class=\"type\">String</span>"),
        Type::Boolean => format!("<span class=\"type\">Boolean</span>"),
        Type::Int32 => format!("<span class=\"type\">Int32</span>"),
        Type::Int64 => format!("<span class=\"type\">Int64</span>"),
        Type::Float32 => format!("<span class=\"type\">Float32</span>"),
        Type::Float64 => format!("<span class=\"type\">Float64</span>"),
        Type::Map(key, value) => {
            format!(
                "<span class=\"type\">Map</span><{}, {}>",
                render_type(key),
                render_type(value)
            )
        }
        Type::Result(error, value) => {
            format!(
                "<span class=\"type\">Result</span><{}, {}>",
                render_type(error),
                render_type(value)
            )
        }
        Type::List(value) => format!("<span class=\"type\">List</span><{}>", render_type(value)),
        Type::Set(_) => "Set".to_string(),
        Type::Option(value) => format!("{}?", render_type(value)),
        Type::Ref(name, _) => format!(
            "<a href=\"#{}\" class=\"type type--custom\">{}</a>",
            name, name
        ),
    }
}

pub fn render_record(record: &Record) -> String {
    let props = record
        .properties
        .iter()
        .map(|prop| format!("    {}: {},\n", prop.name.value, render_type(&prop.type_)))
        .join("");

    format!(
        "<span class=\"keyword\">data</span> {} {{\n{}}}",
        record.name.value, props
    )
}

pub fn render_enum(record: &Enum) -> String {
    let variants = record
        .variants
        .iter()
        .sorted_by_key(|x| x.name.value.clone())
        .map(|variant| {
            if variant.properties.is_empty() {
                format!("    {},", variant.name.value)
            } else {
                let props = variant
                    .properties
                    .iter()
                    .map(|prop| {
                        format!(
                            "        {}: {},\n",
                            prop.name.value,
                            render_type(&prop.type_)
                        )
                    })
                    .join("");

                format!("    {} {{\n{}    }},", variant.name.value, props)
            }
        })
        .join("\n");

    format!(
        "<span class=\"keyword\">enum</span> {} {{\n{}\n}}",
        record.name.value, variants
    )
}

/// Render the
pub fn render(module: &Module) -> String {
    let doc = DocTemplate { module: module };
    doc.render().unwrap()
}
