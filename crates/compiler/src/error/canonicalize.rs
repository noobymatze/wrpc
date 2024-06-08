use serde::{Deserialize, Serialize};

use crate::{ast::source::Name, reporting::Region};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Error {
    BadRecord(Name, Record),
    BadEnum(Name, Enum),
    BadService(Name, Service),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Record {
    BadProperty(Name, Property),
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Enum {
    BadVariant(Name, Variant),
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Variant {
    BadProperty(Name, Property),
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Service {
    BadMethod(Name, Method),
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Method {
    BadParameter(Name, Parameter),
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Parameter {
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Property {
    BadAnnotation(Annotation),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum Annotation {
    UnknownSymbol(Region, String),
    Empty(Region),
    InvalidAnnotation(Region),
}
