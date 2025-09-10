mod model;

use pest::Parser as _;
use pest::iterators::Pair;
use pest_derive::Parser;
use std::fmt;
use std::fs;
use std::path::Path;

pub use model::*;

#[derive(Parser)]
#[grammar = "resources/proto.pest"] // Path relative to the crate root
pub struct ProtoParser;

#[derive(Debug)]
pub enum ParseError {
    Io(std::io::Error),
    Pest(Box<pest::error::Error<Rule>>),
    Message(&'static str),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::Io(e) => write!(f, "IO error: {}", e),
            ParseError::Pest(e) => write!(f, "Parse error: {}", e),
            ParseError::Message(m) => write!(f, "{}", m),
        }
    }
}

impl std::error::Error for ParseError {}

impl From<std::io::Error> for ParseError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}
impl From<pest::error::Error<Rule>> for ParseError {
    fn from(e: pest::error::Error<Rule>) -> Self {
        Self::Pest(Box::new(e))
    }
}

// Public API: parse a .proto file into ProtoModel IR
pub fn parse_proto_file<P: AsRef<Path>>(path: P) -> Result<ProtoModel, ParseError> {
    let content = fs::read_to_string(path)?;
    let mut pairs = ProtoParser::parse(Rule::proto, &content)?;
    let proto_pair = pairs
        .next()
        .ok_or(ParseError::Message("expected proto root"))?;
    Ok(parse_proto(proto_pair))
}

fn parse_proto(pair: Pair<Rule>) -> ProtoModel {
    let mut model = ProtoModel::default();
    let mut types: Vec<TypeDecl> = Vec::new();

    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::package_statement => {
                // package_statement = { "package" ~ package_name ~ ";" }
                let pkg = inner
                    .into_inner()
                    .find(|p| p.as_rule() == Rule::package_name)
                    .and_then(|p| p.into_inner().find(|x| x.as_rule() == Rule::full_ident))
                    .map(|p| p.as_str().to_string());
                model.package = pkg;
            }
            Rule::proto_body => {
                // Unwrap proto_body -> may contain top_level_definition etc.
                for b in inner.into_inner() {
                    match b.as_rule() {
                        Rule::package_statement => {
                            let pkg = b
                                .into_inner()
                                .find(|p| p.as_rule() == Rule::package_name)
                                .and_then(|p| {
                                    p.into_inner().find(|x| x.as_rule() == Rule::full_ident)
                                })
                                .map(|p| p.as_str().to_string());
                            model.package = pkg;
                        }
                        Rule::top_level_definition => {
                            for def in b.into_inner() {
                                match def.as_rule() {
                                    Rule::message_block => {
                                        parse_message_block(def, None, &mut types)
                                    }
                                    Rule::enum_block => parse_enum_block(def, None, &mut types),
                                    _ => {}
                                }
                            }
                        }
                        Rule::message_block => parse_message_block(b, None, &mut types),
                        Rule::enum_block => parse_enum_block(b, None, &mut types),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    model.types = types;
    model
}

fn qualify(parent: Option<&str>, name: &str) -> String {
    if let Some(p) = parent {
        format!("{}.{name}", p)
    } else {
        name.to_string()
    }
}

fn parse_message_block(block: Pair<Rule>, parent: Option<&str>, types: &mut Vec<TypeDecl>) {
    use std::collections::HashSet;
    // message_block = { "message" ~ message_name ~ message_body }
    let mut name: Option<String> = None;
    let mut message = Message::default();

    let mut body_opt: Option<Pair<Rule>> = None;

    for p in block.into_inner() {
        match p.as_rule() {
            Rule::message_name => {
                name = Some(p.as_str().to_string());
            }
            Rule::message_body => {
                body_opt = Some(p);
            }
            _ => {}
        }
    }

    let raw_name = name.unwrap_or_default();
    message.name = qualify(parent, &raw_name);

    // Track nested type names to qualify field references when needed
    let mut nested_names: HashSet<String> = HashSet::new();

    if let Some(body) = body_opt.clone() {
        for elem in body.clone().into_inner() {
            if elem.as_rule() == Rule::message_element {
                for inner in elem.into_inner() {
                    match inner.as_rule() {
                        Rule::enum_block => {
                            // Extract enum name
                            for p in inner.clone().into_inner() {
                                if p.as_rule() == Rule::enum_name {
                                    nested_names.insert(p.as_str().to_string());
                                }
                            }
                        }
                        Rule::message_block => {
                            for p in inner.clone().into_inner() {
                                if p.as_rule() == Rule::message_name {
                                    nested_names.insert(p.as_str().to_string());
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    if let Some(body) = body_opt {
        for elem in body.into_inner() {
            if elem.as_rule() == Rule::message_element {
                for inner in elem.into_inner() {
                    match inner.as_rule() {
                        Rule::field => {
                            if let Some(field) =
                                parse_field(inner, Some(&message.name), Some(&nested_names))
                            {
                                message.fields.push(field);
                            }
                        }
                        Rule::oneof => {
                            // Flatten oneof fields into message fields (no grouping in IR)
                            for oneof_inner in inner.into_inner() {
                                if oneof_inner.as_rule() == Rule::oneof_field {
                                    for f in oneof_inner.into_inner() {
                                        if f.as_rule() == Rule::field
                                            && let Some(field) = parse_field(
                                                f,
                                                Some(&message.name),
                                                Some(&nested_names),
                                            )
                                        {
                                            message.fields.push(field);
                                        }
                                    }
                                }
                            }
                        }
                        Rule::enum_block => parse_enum_block(inner, Some(&message.name), types),
                        Rule::message_block => {
                            parse_message_block(inner, Some(&message.name), types)
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    types.push(TypeDecl::Message(message));
}

fn parse_enum_block(block: Pair<Rule>, parent: Option<&str>, types: &mut Vec<TypeDecl>) {
    // enum_block = { "enum" ~ enum_name ~ "{" ~ enum_body* ~ "}" ~ ";"? }
    let mut name: Option<String> = None;
    let mut en = Enum::default();

    for p in block.into_inner() {
        match p.as_rule() {
            Rule::enum_name => {
                name = Some(p.as_str().to_string());
            }
            Rule::enum_body => {
                for eb in p.into_inner() {
                    if eb.as_rule() == Rule::enum_field {
                        let mut val_name: Option<String> = None;
                        let mut number: Option<i32> = None;
                        for ef in eb.into_inner() {
                            match ef.as_rule() {
                                Rule::enum_field_name => val_name = Some(ef.as_str().to_string()),
                                Rule::enum_field_value => {
                                    // enum_field_value = integer_value
                                    let n = parse_integer_value(ef);
                                    number = Some(n as i64 as i32);
                                }
                                _ => {}
                            }
                        }
                        if let (Some(vn), Some(num)) = (val_name, number) {
                            en.values.push(EnumValue {
                                name: vn,
                                number: num,
                            });
                        }
                    }
                }
            }
            _ => {}
        }
    }

    let raw_name = name.unwrap_or_default();
    en.name = qualify(parent, &raw_name);
    types.push(TypeDecl::Enum(en));
}

fn parse_field(
    pair: Pair<Rule>,
    parent_scope: Option<&str>,
    nested: Option<&std::collections::HashSet<String>>,
) -> Option<Field> {
    // field = { field_modifier? ~ type_reference ~ field_name ~ "=" ~ tag ~ field_options? ~ ";" }
    let mut ty_opt: Option<FieldType> = None;
    let mut name_opt: Option<String> = None;
    let mut order_opt: Option<u32> = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::field_modifier => { /* ignored in IR */ }
            Rule::type_reference => ty_opt = Some(parse_type_reference(p)),
            Rule::field_name => name_opt = Some(p.as_str().to_string()),
            Rule::tag => {
                let n = parse_integer_value(p);
                order_opt = u32::try_from(n).ok();
            }
            _ => {}
        }
    }

    // Qualify custom type if it references a nested declaration in current scope
    if let (Some(FieldType::Custom(tn)), Some(scope)) = (&mut ty_opt, parent_scope)
        && let Some(nested_set) = nested
        && !tn.contains('.')
        && nested_set.contains(tn)
    {
        *tn = qualify(Some(scope), tn);
    }

    match (ty_opt, name_opt, order_opt) {
        (Some(ty), Some(name), Some(order)) => Some(Field { ty, name, order }),
        _ => None,
    }
}

fn parse_type_reference(pair: Pair<Rule>) -> FieldType {
    // type_reference = { scalar_type | _dot? ~ full_ident }
    // If it's a scalar, inner will include a scalar_type; otherwise, we can use the full string slice.
    let mut inners = pair.clone().into_inner();
    if let Some(first) = inners.next()
        && first.as_rule() == Rule::scalar_type
    {
        return FieldType::Scalar(parse_scalar_type(first.as_str()));
    }
    let mut s = pair.as_str().to_string();
    if s.starts_with('.') {
        s.remove(0);
    }
    FieldType::Custom(s)
}

fn parse_scalar_type(s: &str) -> ScalarType {
    match s {
        "double" => ScalarType::Double,
        "float" => ScalarType::Float,
        "int32" => ScalarType::Int32,
        "int64" => ScalarType::Int64,
        "uint32" => ScalarType::Uint32,
        "uint64" => ScalarType::Uint64,
        "sint32" => ScalarType::Sint32,
        "sint64" => ScalarType::Sint64,
        "fixed32" => ScalarType::Fixed32,
        "fixed64" => ScalarType::Fixed64,
        "sfixed32" => ScalarType::Sfixed32,
        "sfixed64" => ScalarType::Sfixed64,
        "bool" => ScalarType::Bool,
        "string" => ScalarType::String,
        "bytes" => ScalarType::Bytes,
        _ => ScalarType::String, // fallback shouldn't happen
    }
}

fn parse_integer_value(pair: Pair<Rule>) -> u64 {
    // integer_value = dec|hex|oct with optional minus; return as unsigned (negative saturates to 0)
    let s = pair.as_str();
    let (neg, rest) = if let Some(stripped) = s.strip_prefix('-') {
        (true, stripped)
    } else {
        (false, s)
    };
    let val: i128 = if rest.starts_with("0x") || rest.starts_with("0X") {
        i128::from_str_radix(&rest[2..], 16).unwrap_or(0)
    } else if rest.starts_with('0') && rest.len() > 1 {
        // Octal per grammar
        i128::from_str_radix(&rest[1..], 8).unwrap_or(0)
    } else {
        rest.parse::<i128>().unwrap_or(0)
    };
    let signed = if neg { -val } else { val };
    if signed < 0 { 0 } else { signed as u64 }
}

// Test module.
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn get_message<'a>(model: &'a ProtoModel, name: &str) -> &'a Message {
        model
            .types
            .iter()
            .find_map(|t| match t {
                TypeDecl::Message(m) if m.name == name => Some(m),
                _ => None,
            })
            .expect("message not found")
    }

    fn get_enum<'a>(model: &'a ProtoModel, name: &str) -> &'a Enum {
        model
            .types
            .iter()
            .find_map(|t| match t {
                TypeDecl::Enum(e) if e.name == name => Some(e),
                _ => None,
            })
            .expect("enum not found")
    }

    #[test]
    fn parses_order_proto_model() {
        let model = parse_proto_file("tests/resources/order.proto").expect("parse failed");
        assert_eq!(model.package.as_deref(), Some("me.alekseinovikov.proto"));

        // Order message
        let order = get_message(&model, "Order");
        let f = |n: &str| order.fields.iter().find(|f| f.name == n).unwrap();
        assert_eq!(f("id").order, 1);
        assert!(matches!(f("id").ty, FieldType::Scalar(ScalarType::Int32)));
        assert!(matches!(
            f("name").ty,
            FieldType::Scalar(ScalarType::String)
        ));
        assert_eq!(f("name").order, 2);
        assert_eq!(f("items").order, 3);
        assert!(matches!(f("items").ty, FieldType::Custom(ref s) if s == "OrderItem"));
        assert_eq!(f("shipping_address").order, 4);
        assert!(
            matches!(f("shipping_address").ty, FieldType::Custom(ref s) if s == "Order.Address")
        );
        assert_eq!(f("status").order, 5);
        assert!(matches!(f("status").ty, FieldType::Custom(ref s) if s == "Order.Status"));

        // Nested Order.Address
        let addr = get_message(&model, "Order.Address");
        let f2 = |n: &str| addr.fields.iter().find(|f| f.name == n).unwrap();
        assert!(matches!(
            f2("street").ty,
            FieldType::Scalar(ScalarType::String)
        ));
        assert!(matches!(
            f2("city").ty,
            FieldType::Scalar(ScalarType::String)
        ));

        // OrderItem message
        let item = get_message(&model, "OrderItem");
        let fi = |n: &str| item.fields.iter().find(|f| f.name == n).unwrap();
        assert!(matches!(
            fi("name").ty,
            FieldType::Scalar(ScalarType::String)
        ));
        assert!(matches!(
            fi("count").ty,
            FieldType::Scalar(ScalarType::Int64)
        ));
        assert!(matches!(fi("type").ty, FieldType::Custom(ref s) if s == "OrderItemType"));
        assert!(matches!(
            fi("price_decimal").ty,
            FieldType::Scalar(ScalarType::Double)
        ));
        assert!(matches!(
            fi("price_cents").ty,
            FieldType::Scalar(ScalarType::Int64)
        ));

        // Enums
        let status = get_enum(&model, "Order.Status");
        let mut status_vals: Vec<(String, i32)> = status
            .values
            .iter()
            .map(|v| (v.name.clone(), v.number))
            .collect();
        status_vals.sort_by_key(|(_, n)| *n);
        assert_eq!(
            status_vals,
            vec![
                ("NEW".to_string(), 0),
                ("PAID".to_string(), 1),
                ("SHIPPED".to_string(), 2)
            ]
        );

        let item_type = get_enum(&model, "OrderItemType");
        let mut item_type_vals: Vec<(String, i32)> = item_type
            .values
            .iter()
            .map(|v| (v.name.clone(), v.number))
            .collect();
        item_type_vals.sort_by_key(|(_, n)| *n);
        assert_eq!(
            item_type_vals,
            vec![("REGULAR".to_string(), 0), ("DISCOUNT".to_string(), 1)]
        );
    }

    #[test]
    fn invalid_syntax_returns_error() {
        let content = r#"
            syntax = "proto3";
            message Order {
                int32 id = 1
                string name = 2;
            }
        "#;
        fs::create_dir_all("target/tmp").unwrap();
        fs::write("target/tmp/invalid.proto", content).unwrap();
        let result = parse_proto_file("target/tmp/invalid.proto");
        assert!(result.is_err());
    }

    #[test]
    fn missing_file_returns_io_error() {
        let result = parse_proto_file("tests/resources/__missing.proto");
        assert!(matches!(result, Err(ParseError::Io(_))));
    }
}
