#![allow(dead_code)]

// Minimal IR for .proto files per current requirements.
// - Top-level has package and a list of types (message or enum only).
// - Message contains only fields.
// - Field type is either a scalar or a custom type.
// - Field has a name and order (tag).

#[derive(Debug, Clone, PartialEq, Default)]
pub struct ProtoModel {
    pub package: Option<String>,
    pub types: Vec<TypeDecl>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeDecl {
    Message(Message),
    Enum(Enum),
}

// ---------------- Message & Fields ----------------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Message {
    pub name: String,
    pub fields: Vec<Field>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub ty: FieldType,
    pub name: String,
    pub order: u32, // tag number
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldType {
    Scalar(ScalarType),
    Custom(String), // fully-qualified or simple type name
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScalarType {
    Double,
    Float,
    Int32,
    Int64,
    Uint32,
    Uint64,
    Sint32,
    Sint64,
    Fixed32,
    Fixed64,
    Sfixed32,
    Sfixed64,
    Bool,
    String,
    Bytes,
}

// ---------------- Enum ----------------

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Enum {
    pub name: String,
    pub values: Vec<EnumValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumValue {
    pub name: String,
    pub number: i32,
}
