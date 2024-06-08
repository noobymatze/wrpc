use crate::ast::constraints::Constraint;
use crate::ast::source::Decl;
use crate::ast::{canonical as can, source as src};
use crate::error::canonicalize;
use std::collections::HashMap;

pub fn canonicalize(module: &src::Module) -> Result<can::Module, Vec<canonicalize::Error>> {
    let mut records = HashMap::new();
    let mut enums = HashMap::new();
    let mut services = HashMap::new();
    let mut errors = vec![];
    for decl in module.declarations.iter() {
        match decl {
            Decl::Data(data) => match canonicalize_data(data) {
                Ok(record) => {
                    records.insert(record.name.value.clone(), record);
                }
                Err(record_errors) => {
                    let mut record_errors = record_errors
                        .iter()
                        .map(|error| {
                            canonicalize::Error::BadRecord(data.name.clone(), error.clone())
                        })
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut record_errors);
                }
            },
            Decl::Enum(data) => match canonicalize_enum(data) {
                Ok(enum_value) => {
                    enums.insert(data.name.value.clone(), enum_value);
                }
                Err(enum_errors) => {
                    let mut enum_errors = enum_errors
                        .iter()
                        .map(|error| canonicalize::Error::BadEnum(data.name.clone(), error.clone()))
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut enum_errors);
                }
            },
            Decl::Service(service) => match canonicalize_service(service) {
                Ok(record) => {
                    services.insert(record.name.value.clone(), record);
                }
                Err(record_errors) => {
                    let mut record_errors = record_errors
                        .iter()
                        .map(|error| {
                            canonicalize::Error::BadService(service.name.clone(), error.clone())
                        })
                        .collect::<Vec<canonicalize::Error>>();
                    errors.append(&mut record_errors);
                }
            },
        }
    }

    if errors.is_empty() {
        Ok(can::Module {
            records,
            services,
            enums,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_data(data: &src::Data) -> Result<can::Record, Vec<canonicalize::Record>> {
    let mut properties = vec![];
    let mut errors = vec![];
    for property in &data.properties {
        match canonicalize_annotations(&property.annotations) {
            Err(annotation_errors) => {
                let mut annotation_errors = annotation_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Record::BadProperty(
                            property.name.clone(),
                            canonicalize::Property::BadAnnotation(error.clone()),
                        )
                    })
                    .collect::<Vec<canonicalize::Record>>();
                errors.append(&mut annotation_errors);
            }
            Ok(annotations) => properties.push(can::Property {
                comment: property.doc_comment.clone(),
                annotations,
                name: property.name.clone(),
                type_: parse_type(&property.type_),
            }),
        };
    }

    let record_annotations =
        canonicalize_annotations(&data.annotations).map_err(|annotation_errors| {
            annotation_errors
                .iter()
                .map(|error| canonicalize::Record::BadAnnotation(error.clone()))
                .collect::<Vec<canonicalize::Record>>()
        });

    match record_annotations {
        Ok(annotations) => {
            if errors.is_empty() {
                Ok(can::Record {
                    annotations,
                    comment: data.doc_comment.clone(),
                    name: data.name.clone(),
                    properties,
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

fn canonicalize_annotations(
    annotations: &Vec<src::Annotation>,
) -> Result<Vec<can::Annotation>, Vec<canonicalize::Annotation>> {
    let mut canonical_annotations = vec![];
    let mut errors = vec![];
    for annotation in annotations {
        match parse_annotation(annotation) {
            Ok(annotation) => canonical_annotations.push(annotation),
            Err(error) => errors.push(error),
        }
    }

    if errors.is_empty() {
        Ok(canonical_annotations)
    } else {
        Err(errors)
    }
}

fn parse_annotation(
    annotation: &src::Annotation,
) -> Result<can::Annotation, canonicalize::Annotation> {
    match &annotation.expr {
        src::Expr::List(region, expressions) => match expressions.as_slice() {
            [] => Err(canonicalize::Annotation::Empty(region.clone())),
            [src::Expr::Symbol(_, value), args @ ..] if value == "check" => {
                let constraints = parse_constraints(args)?;
                Ok(can::Annotation::Check(constraints))
            }
            //[src::Expr::Symbol(region, value), _args @ ..] => Err(
            //    canonicalize::Annotation::UnknownSymbol(region.clone(), value.clone()),
            //),
            _ => Ok(can::Annotation::Custom(canonicalize_expr(&annotation.expr))),
        },
        _ => Ok(can::Annotation::Custom(canonicalize_expr(&annotation.expr))),
    }
}

fn parse_constraints(args: &[src::Expr]) -> Result<Vec<Constraint>, canonicalize::Annotation> {
    let mut constraints = vec![];
    for expr in args {
        constraints.push(parse_constraint(expr)?);
    }
    Ok(constraints)
}

fn parse_constraint(expr: &src::Expr) -> Result<Constraint, canonicalize::Annotation> {
    let value = match expr {
        src::Expr::Boolean(_, value) => Constraint::Boolean(*value),
        src::Expr::Number(_, value) => Constraint::Number(*value),
        src::Expr::String(_, value) => Constraint::String(value.clone()),
        src::Expr::Symbol(region, value) => {
            if value.starts_with(".") {
                let (_, value) = value.split_at(1);
                Constraint::Ref(value.to_owned())
            } else {
                return Err(canonicalize::Annotation::UnknownSymbol(
                    region.clone(),
                    value.clone(),
                ));
            }
        }
        src::Expr::Keyword(_, value) => Constraint::String(value.clone()),
        src::Expr::List(_, expressions) => match expressions.as_slice() {
            [src::Expr::Symbol(_, value), args @ ..] if value == "<" => Constraint::Lt(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == "<=" => Constraint::Le(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == "=" => Constraint::Eq(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == ">=" => Constraint::Ge(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            [src::Expr::Symbol(_, value), args @ ..] if value == ">" => Constraint::Gt(
                args.iter()
                    .map(parse_constraint)
                    .collect::<Result<Vec<Constraint>, canonicalize::Annotation>>()?,
            ),
            _ => unimplemented!(),
        },
        src::Expr::Map(_, values) => Constraint::Map(
            values
                .iter()
                .map(|(a, b)| {
                    parse_constraint(a)
                        .and_then(|key| parse_constraint(b).map(|value| (key, value)))
                })
                .collect::<Result<Vec<(Constraint, Constraint)>, canonicalize::Annotation>>()?,
        ),
    };

    Ok(value)
}

fn canonicalize_expr(expr: &src::Expr) -> can::Expr {
    match expr {
        src::Expr::Boolean(region, value) => can::Expr::Boolean(region.clone(), value.clone()),
        src::Expr::Number(region, value) => can::Expr::Number(region.clone(), value.clone()),
        src::Expr::Keyword(region, value) => can::Expr::Keyword(region.clone(), value.clone()),
        src::Expr::String(region, value) => can::Expr::String(region.clone(), value.clone()),
        src::Expr::Symbol(region, value) => can::Expr::Symbol(region.clone(), value.clone()),
        src::Expr::Map(region, value) => can::Expr::Map(
            region.clone(),
            value
                .iter()
                .map(|(key, value)| (canonicalize_expr(key), canonicalize_expr(value)))
                .collect(),
        ),
        src::Expr::List(region, value) => can::Expr::List(
            region.clone(),
            value.iter().map(canonicalize_expr).collect(),
        ),
    }
}

fn canonicalize_enum(data: &src::Enum) -> Result<can::Enum, Vec<canonicalize::Enum>> {
    let mut errors = vec![];
    let mut variants = vec![];

    let (annotations, mut an_errors) = match canonicalize_annotations(&data.annotations) {
        Ok(annotations) => (annotations, vec![]),
        Err(errors) => {
            let errors = errors
                .iter()
                .map(|error| canonicalize::Enum::BadAnnotation(error.clone()))
                .collect();
            (vec![], errors)
        }
    };

    for variant in &data.variants {
        match canonicalize_variant(variant) {
            Ok(variant) => variants.push(variant),
            Err(variant_errors) => {
                let mut variant_errors = variant_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Enum::BadVariant(variant.name.clone(), error.clone())
                    })
                    .collect();
                errors.append(&mut variant_errors);
            }
        }
    }

    errors.append(&mut an_errors);

    if errors.is_empty() {
        Ok(can::Enum {
            annotations,
            comment: data.doc_comment.clone(),
            name: data.name.clone(),
            variants,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_variant(
    variant: &src::Variant,
) -> Result<can::Variant, Vec<canonicalize::Variant>> {
    let mut properties = vec![];
    let mut errors = vec![];
    for property in &variant.properties {
        match canonicalize_annotations(&property.annotations) {
            Err(annotation_errors) => {
                let mut annotation_errors = annotation_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Variant::BadProperty(
                            property.name.clone(),
                            canonicalize::Property::BadAnnotation(error.clone()),
                        )
                    })
                    .collect::<Vec<canonicalize::Variant>>();
                errors.append(&mut annotation_errors);
            }
            Ok(annotations) => properties.push(can::Property {
                comment: property.doc_comment.clone(),
                annotations,
                name: property.name.clone(),
                type_: parse_type(&property.type_),
            }),
        };
    }

    let record_annotations =
        canonicalize_annotations(&variant.annotations).map_err(|annotation_errors| {
            annotation_errors
                .iter()
                .map(|error| canonicalize::Variant::BadAnnotation(error.clone()))
                .collect::<Vec<canonicalize::Variant>>()
        });

    match record_annotations {
        Ok(annotations) => {
            if errors.is_empty() {
                Ok(can::Variant {
                    annotations,
                    comment: variant.doc_comment.clone(),
                    name: variant.name.clone(),
                    properties,
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

fn canonicalize_service(
    service: &src::Service,
) -> Result<can::Service, Vec<canonicalize::Service>> {
    let mut methods = HashMap::new();
    let mut errors = vec![];
    let (annotations, mut an_errors) = match canonicalize_annotations(&service.annotations) {
        Ok(annotations) => (annotations, vec![]),
        Err(errors) => {
            let errors = errors
                .iter()
                .map(|error| canonicalize::Service::BadAnnotation(error.clone()))
                .collect();
            (vec![], errors)
        }
    };

    errors.append(&mut an_errors);

    for method in &service.methods {
        match canonicalize_method(method) {
            Ok(method) => {
                methods.insert(method.name.value.clone(), method);
            }
            Err(method_errors) => {
                let mut method_errors = method_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Service::BadMethod(method.name.clone(), error.clone())
                    })
                    .collect::<Vec<canonicalize::Service>>();
                errors.append(&mut method_errors);
            }
        }
    }

    if errors.is_empty() {
        Ok(can::Service {
            annotations,
            name: service.name.clone(),
            comment: service.doc_comment.clone(),
            methods,
        })
    } else {
        Err(errors)
    }
}

fn canonicalize_method(method: &src::Method) -> Result<can::Method, Vec<canonicalize::Method>> {
    let mut parameters = vec![];
    let mut errors = vec![];
    for parameter in &method.parameters {
        match canonicalize_annotations(&parameter.annotations) {
            Err(annotation_errors) => {
                let mut annotation_errors = annotation_errors
                    .iter()
                    .map(|error| {
                        canonicalize::Method::BadParameter(
                            parameter.name.clone(),
                            canonicalize::Parameter::BadAnnotation(error.clone()),
                        )
                    })
                    .collect::<Vec<canonicalize::Method>>();
                errors.append(&mut annotation_errors);
            }
            Ok(annotations) => parameters.push(can::Parameter {
                comment: None,
                annotations,
                name: parameter.name.clone(),
                type_: parse_type(&parameter.type_),
            }),
        };
    }

    let record_annotations =
        canonicalize_annotations(&method.annotations).map_err(|annotation_errors| {
            annotation_errors
                .iter()
                .map(|error| canonicalize::Method::BadAnnotation(error.clone()))
                .collect::<Vec<canonicalize::Method>>()
        });

    match record_annotations {
        Ok(annotations) => {
            if errors.is_empty() {
                Ok(can::Method {
                    annotations,
                    comment: method.doc_comment.clone(),
                    name: method.name.clone(),
                    return_type: method.return_type.clone().map(|type_| parse_type(&type_)),
                    parameters,
                })
            } else {
                Err(errors)
            }
        }
        Err(mut annotation_errors) => {
            errors.append(&mut annotation_errors);
            Err(errors)
        }
    }
}

fn parse_type(type_: &src::Type) -> can::Type {
    match type_.name.value.as_str() {
        "String" => can::Type::String,
        "Int32" => can::Type::Int32,
        "Int64" => can::Type::Int64,
        "Float32" => can::Type::Float32,
        "Float64" => can::Type::Float64,
        "Boolean" => can::Type::Boolean,
        "Map" => {
            let key_type = parse_type(&type_.variables[0]);
            let value_type = parse_type(&type_.variables[1]);
            can::Type::Map(key_type.into(), value_type.into())
        }
        "Set" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::Set(result.into())
        }
        "List" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::List(result.into())
        }
        "Option" => {
            let result = parse_type(&type_.variables[0]);
            can::Type::Option(result.into())
        }
        "Result" => {
            let error = parse_type(&type_.variables[0]);
            let value = parse_type(&type_.variables[1]);
            can::Type::Result(error.into(), value.into())
        }
        _ => can::Type::Ref(type_.name.value.clone()),
    }
}
